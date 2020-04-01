use super::*;
use crate::Exception;
use std::marker::PhantomData;
use terminus_spaceport::memory::region::{U32Access, U64Access};
use std::convert::TryFrom;

mod pmp;

use pmp::*;

mod pte;

use pte::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum MmuOpt {
    Load,
    Store,
    Fetch,
}

impl MmuOpt {
    fn access_exception(&self, addr: RegT) -> Exception {
        match self {
            MmuOpt::Fetch => Exception::FetchAccess(addr as u64),
            MmuOpt::Load => Exception::LoadAccess(addr as u64),
            MmuOpt::Store => Exception::StoreAccess(addr as u64)
        }
    }

    fn pagefault_exception(&self, addr: RegT) -> Exception {
        match self {
            MmuOpt::Fetch => Exception::FetchPageFault(addr as u64),
            MmuOpt::Load => Exception::LoadPageFault(addr as u64),
            MmuOpt::Store => Exception::StorePageFault(addr as u64)
        }
    }

    fn pmp_match(&self, pmpcfg: &PmpCfgEntry) -> bool {
        match self {
            MmuOpt::Fetch => pmpcfg.x() == 1,
            MmuOpt::Load => pmpcfg.r() == 1,
            MmuOpt::Store => pmpcfg.w() == 1
        }
    }
}

pub struct Mmu {
    p: Rc<ProcessorState>,
}

impl Mmu {
    pub fn new(p: &Rc<ProcessorState>) -> Mmu {
        Mmu {
            p: p.clone(),
        }
    }

    fn pmpcfgs_iter(&self) -> PmpCfgsIter {
        PmpCfgsIter::new(self, PhantomData)
    }

    fn get_pmpaddr(&self, idx: u8) -> RegT {
        self.p.csr().read(0x3b0 + idx as RegT).unwrap()
    }

    fn match_pmpcfg_entry(&self, addr: u64, len: usize) -> Option<PmpCfgEntry> {
        self.pmpcfgs_iter().enumerate()
            .find(|(idx, entry)| {
                ((addr >> 2)..((addr + len as u64 - 1) >> 2) + 1)
                    .map(|trail_addr| {
                        match PmpAType::try_from(entry.a()).unwrap() {
                            PmpAType::OFF => false,
                            PmpAType::TOR => {
                                let low = if *idx == 0 {
                                    0
                                } else {
                                    self.get_pmpaddr((*idx - 1) as u8)
                                };
                                let high = self.get_pmpaddr(*idx as u8);
                                trail_addr >= low && trail_addr < high
                            }
                            PmpAType::NA4 => {
                                let pmpaddr = self.get_pmpaddr(*idx as u8);
                                trail_addr == pmpaddr
                            }
                            PmpAType::NAPOT => {
                                let pmpaddr = self.get_pmpaddr(*idx as u8);
                                let trialing_ones = (!pmpaddr).trailing_zeros();
                                (trail_addr >> trialing_ones) == (pmpaddr >> trialing_ones)
                            }
                        }
                    })
                    .fold(true, |acc, m| { acc && m })
            })
            .map(|(_, entry)| { entry })
    }

    fn check_pmp(&self, addr: u64, len: usize, opt: MmuOpt, privilege: Privilege) -> bool {
        if let Some(entry) = self.match_pmpcfg_entry(addr, len) {
            privilege == Privilege::M && entry.l() == 0 || opt.pmp_match(&entry)
        } else {
            privilege == Privilege::M
        }
    }

    fn pte_info(&self) -> PteInfo {
        PteInfo::new(&self.p.csr().satp)
    }

    fn get_privileage(&self, opt: MmuOpt) -> Privilege {
        if self.p.csr().mstatus.mprv() == 1 && opt != MmuOpt::Fetch {
            Privilege::try_from(self.p.csr().mstatus.mpp() as u8).unwrap()
        } else {
            self.p.privilege.borrow().clone()
        }
    }


    fn check_pte_privilege(&self, addr: RegT, pte_attr: &PteAttr, opt: &MmuOpt, privilege: &Privilege) -> Result<(), Exception> {
        let priv_s = *privilege == Privilege::S;
        let pte_x = pte_attr.x() == 1;
        let pte_u = pte_attr.u() == 1;
        let pte_r = pte_attr.r() == 1;
        let pte_w = pte_attr.w() == 1;
        let mxr = self.p.csr().mstatus.mxr() == 1;
        let sum = self.p.csr().mstatus.sum() == 1;
        match opt {
            MmuOpt::Fetch => {
                if !pte_x || pte_u == priv_s {
                    return Err(opt.pagefault_exception(addr));
                }
            }
            MmuOpt::Load => {
                if priv_s && !sum && pte_u || !pte_u && !priv_s || !pte_r && !mxr || mxr && !pte_r && !pte_x {
                    return Err(opt.pagefault_exception(addr));
                }
            }
            MmuOpt::Store => {
                if priv_s && !sum && pte_u || !pte_u && !priv_s || !pte_w || !pte_r {
                    return Err(opt.pagefault_exception(addr));
                }
            }
        }
        Ok(())
    }

    fn pt_walk(&self, va: RegT, opt: MmuOpt, privilege: Privilege) -> Result<u64, Exception> {
        if privilege == Privilege::M {
            return Ok(va as u64);
        }
        let info = self.pte_info();
        if info.mode == PteMode::Bare {
            return Ok(va as u64);
        }
        let vaddr = Vaddr::new(&info.mode, va);
        //step 1
        let ppn = self.p.csr().satp.ppn();
        let mut a = ppn * info.page_size as RegT;
        let mut level = info.level - 1;
        let mut leaf_pte: Pte;
        let mut pte_addr: u64;
        loop {
            //step 2
            pte_addr = (a + vaddr.vpn(level).unwrap() * info.size as RegT) as u64;
            if !self.check_pmp(pte_addr, info.size, MmuOpt::Load, Privilege::S) {
                return Err(opt.access_exception(va));
            }
            let pte = match Pte::load(&info, &self.p.bus, pte_addr) {
                Ok(pte) => pte,
                Err(_) => return Err(opt.access_exception(va))
            };
            //step 3
            if pte.attr().v() == 0 || pte.attr().r() == 0 && pte.attr().w() == 1 {
                return Err(opt.pagefault_exception(va));
            }
            //step 4
            if pte.attr().r() == 1 || pte.attr().x() == 1 {
                leaf_pte = pte;
                break;
            } else if level == 0 {
                return Err(opt.pagefault_exception(va));
            } else {
                level -= 1;
                a = pte.ppn_all() * info.page_size as RegT;
            }
        }
        //step 5
        self.check_pte_privilege(va, &leaf_pte.attr(), &opt, &privilege)?;
        //step 6
        if level > 0 && leaf_pte.ppn(level - 1).unwrap() != 0 {
            return Err(opt.pagefault_exception(va));
        }
        //step 7
        if leaf_pte.attr().d() == 0 && opt == MmuOpt::Store || leaf_pte.attr().a() == 0 {
            if self.p.config.enabel_dirty {
                let mut new_attr = leaf_pte.attr();
                new_attr.set_a(1);
                new_attr.set_d((opt == MmuOpt::Store) as u8);
                if !self.check_pmp(pte_addr, info.size, MmuOpt::Store, Privilege::S) {
                    return Err(opt.access_exception(va));
                }
                leaf_pte.set_attr(new_attr);
                if leaf_pte.store(&self.p.bus, pte_addr).is_err() {
                    return Err(opt.access_exception(va));
                }
            } else {
                return Err(opt.pagefault_exception(va));
            }
        }
        //step 8
        Ok(Paddr::new(&vaddr, &leaf_pte, &info, level).value() as u64)
    }

    pub fn translate(&self, va: RegT, len: RegT, opt: MmuOpt) -> Result<u64, Exception> {
        let privilege = self.get_privileage(opt);
        match self.pt_walk(va, opt, privilege) {
            Ok(pa) => if !self.check_pmp(pa, len as usize, opt, privilege) {
                return Err(opt.access_exception(va));
            } else {
                Ok(pa)
            }
            Err(e) => Err(e)
        }
    }
}


#[test]
fn pmp_basic_test() {
    let space = Arc::new(Space::new());
    let p = Processor::new(ProcessorCfg { xlen: XLen::X32, enabel_dirty: true }, &space);
    //no valid region
    assert_eq!(p.mmu().match_pmpcfg_entry(0, 1), None);
    //NA4
    p.state.csr_mut().pmpcfg0.set_bit_range(4, 3, PmpAType::NA4.into());
    p.state.csr_mut().pmpaddr0.set(0x8000_0000 >> 2);
    assert!(p.mmu().match_pmpcfg_entry(0x8000_0000, 4).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x8000_0000, 5).is_none());

    //NAPOT
    p.state.csr_mut().pmpcfg3.set_bit_range(4, 3, PmpAType::NAPOT.into());
    p.state.csr_mut().pmpaddr12.set((0x2000_0000 + 0x1_0000 - 1) >> 2);
    assert!(p.mmu().match_pmpcfg_entry(0x2000_0000, 4).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x2000_ffff, 1).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x2000_ffff, 2).is_none());
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2000_ffff, 1), p.mmu().match_pmpcfg_entry(0x2000_0000, 4));
    assert_eq!(p.mmu().match_pmpcfg_entry(0x1000_ffff, 1), None);
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2001_0000, 4), None);
    //TOR
    p.state.csr_mut().pmpcfg3.set_bit_range(12, 11, PmpAType::TOR.into());
    p.state.csr_mut().pmpaddr13.set((0x2000_0000 + 0x1_0000) >> 2);
    p.state.csr_mut().pmpcfg3.set_bit_range(20, 19, PmpAType::TOR.into());
    p.state.csr_mut().pmpaddr14.set((0x2000_0000 + 0x2_0000) >> 2);
    assert!(p.mmu().match_pmpcfg_entry(0x2001_0000, 4).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x2001_ffff, 1).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x2001_ffff, 2).is_none());
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2002_0000, 4), None);
    p.state.csr_mut().pmpcfg3.set_bit_range(23, 23, 1);
    assert!(p.mmu().match_pmpcfg_entry(0x2001_0000, 4).is_some());
}