#![feature(concat_idents)]

use super::*;
use crate::Exception;
use std::marker::PhantomData;
use terminus_global::*;
use terminus_macros::*;
use std::convert::TryFrom;
use num_enum::{IntoPrimitive, TryFromPrimitive};

pub struct Mmu<'p> {
    p: &'p Processor,
    marker: PhantomData<&'p Processor>,
}

impl<'p> Mmu<'p> {
    pub fn new(p: &'p Processor, marker: PhantomData<&'p Processor>) -> Mmu<'p> {
        Mmu {
            p,
            marker,
        }
    }

    fn pmpcfgs_iter<'m>(&'m self) -> PmpCfgsIter<'m, 'p> {
        PmpCfgsIter {
            mmu: &self,
            idx: 0,
            marker: PhantomData,
        }
    }

    fn get_pmpaddr(&self, idx: u8) -> RegT {
        self.p.basic_csr.read(0x3b0 + idx as RegT).unwrap()
    }

    fn match_pmpcfg_entry<F: Fn(&Processor, &PmpCfgEntry) -> bool>(&self, addr: u64, condition: F) -> Option<PmpCfgEntry> {
        let trail_addr = addr >> 2;
        self.pmpcfgs_iter().enumerate()
            .find(|(idx, entry)| {
                condition(self.p, entry) && match PmpAType::try_from(entry.a()).unwrap() {
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
            .map(|(_, entry)| { entry })
    }
}


#[derive(IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u8)]
enum PmpAType {
    OFF = 0,
    TOR = 1,
    NA4 = 2,
    NAPOT = 3,
}

bitfield! {
#[derive(Eq,PartialEq)]
struct PmpCfgEntry(u8);
impl Debug;
r, set_r:0, 0;
w, set_w:1, 1;
x, set_x:2, 2;
a, set_a:4,3;
l, set_l:7, 7;
}

impl From<u8> for PmpCfgEntry {
    fn from(v: u8) -> Self {
        PmpCfgEntry(v)
    }
}

struct PmpCfgsIter<'m, 'p> {
    mmu: &'m Mmu<'p>,
    idx: u8,
    marker: PhantomData<&'m Mmu<'p>>,
}


impl<'a, 'b> PmpCfgsIter<'a, 'b> {
    fn get_cfg(&self) -> &PmpCfg {
        match self.mmu.p.xlen {
            XLen::X32 => {
                match (self.idx >> 2) & 0x3 {
                    0 => &self.mmu.p.basic_csr.pmpcfg0,
                    1 => &self.mmu.p.basic_csr.pmpcfg1,
                    2 => &self.mmu.p.basic_csr.pmpcfg2,
                    3 => &self.mmu.p.basic_csr.pmpcfg3,
                    _ => unreachable!()
                }
            }
            XLen::X64 => {
                match (self.idx >> 3) & 0x1 {
                    0 => &self.mmu.p.basic_csr.pmpcfg0,
                    1 => &self.mmu.p.basic_csr.pmpcfg2,
                    _ => unreachable!()
                }
            }
        }
    }

    fn get_entry(&self) -> PmpCfgEntry {
        let offset: u8 = match self.mmu.p.xlen {
            XLen::X32 => self.idx.bit_range(1, 0),
            XLen::X64 => self.idx.bit_range(2, 0),
        };
        (self.get_cfg().bit_range(((offset as usize) << 3) + 7, (offset as usize) << 3)).into()
    }
}

impl<'a, 'b> Iterator for PmpCfgsIter<'a, 'b> {
    type Item = PmpCfgEntry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == 15 {
            None
        } else {
            let res = self.get_entry();
            self.idx += 1;
            Some(res)
        }
    }
}

#[derive(IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u8)]
enum PteMode {
    Bare = 0,
    Sv32 = 1,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
    Sv64 = 11,
}

struct PteInfo {
    mode: PteMode,
    level: usize,
    size: usize,
}

impl From<Satp> for PteInfo {
    fn from(v: Satp) -> Self {
        match v.xlen {
            XLen::X32 => PteInfo {
                mode: PteMode::try_from(v.mode() as u8).unwrap(),
                level: 2,
                size: 4,
            },
            XLen::X64 => {
                let mode = PteMode::try_from(v.mode() as u8).unwrap();
                let level = match mode {
                    PteMode::Sv39 => 3,
                    PteMode::Sv48 => 4,
                    PteMode::Bare => 0,
                    _ => unreachable!()
                };
                PteInfo {
                    mode,
                    level,
                    size: 8,
                }
            }
        }
    }
}


bitfield! {
#[derive(Eq,PartialEq)]
struct PteAttr(u8);
impl Debug;
v, set_v:0, 0;
r, set_r:1, 1;
w, set_w:2, 2;
x, set_x:3, 3;
u, set_u:4, 4;
g, set_g:5, 5;
a, set_a:6,6;
d, set_d:7, 7;
}


impl From<u8> for PteAttr {
    fn from(v: u8) -> Self {
        PteAttr(v)
    }
}

bitfield! {
struct Sv32Vaddr(RegT);
impl Debug;
vpn1,_:31, 22;
vpn0,_:21, 12;
offset,_:11,0;
}

impl Sv32Vaddr {
    fn vpn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.vpn0()),
            1 => Some(self.vpn1()),
            _ => None
        }
    }
}

bitfield! {
struct Sv32Paddr(RegT);
impl Debug;
ppn1,_:33, 22;
ppn0,_:21, 12;
offset,_:11,0;
}

impl Sv32Paddr {
    fn ppn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.ppn0()),
            1 => Some(self.ppn1()),
            _ => None
        }
    }
}

bitfield! {
struct Sv32Pte(RegT);
impl Debug;
ppn1,_:31, 20;
ppn0,_:19, 10;
rsw,_:9,8;
attr_raw,_:7,0;
}

impl Sv32Pte {
    fn ppn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.ppn0()),
            1 => Some(self.ppn1()),
            _ => None
        }
    }
}

bitfield! {
struct Sv39Vaddr(RegT);
impl Debug;
vpn2,_:38, 30;
vpn1,_:29, 21;
vpn0,_:20, 12;
offset,_:11,0;
}

impl Sv39Vaddr {
    fn vpn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.vpn0()),
            1 => Some(self.vpn1()),
            2 => Some(self.vpn2()),
            _ => None
        }
    }
}

bitfield! {
struct Sv39Paddr(RegT);
impl Debug;
ppn2,_:55, 30;
ppn1,_:29, 21;
ppn0,_:20, 12;
offset,_:11,0;
}

impl Sv39Paddr {
    fn ppn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.ppn0()),
            1 => Some(self.ppn1()),
            2 => Some(self.ppn2()),
            _ => None
        }
    }
}

bitfield! {
struct Sv39Pte(RegT);
impl Debug;
ppn2,_:53, 28;
ppn1,_:27, 19;
ppn0,_:18, 10;
rsw,_:9,8;
attr_raw,_:7,0;
}

impl Sv39Pte {
    fn ppn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.ppn0()),
            1 => Some(self.ppn1()),
            2 => Some(self.ppn2()),
            _ => None
        }
    }
}

bitfield! {
struct Sv48Vaddr(RegT);
impl Debug;
vpn3,_:47, 39;
vpn2,_:38, 30;
vpn1,_:29, 21;
vpn0,_:20, 12;
offset,_:11,0;
}

impl Sv48Vaddr {
    fn vpn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.vpn0()),
            1 => Some(self.vpn1()),
            2 => Some(self.vpn2()),
            2 => Some(self.vpn3()),
            _ => None
        }
    }
}

bitfield! {
struct Sv48Paddr(RegT);
impl Debug;
ppn3,_:55, 39;
ppn2,_:38, 30;
ppn1,_:29, 21;
ppn0,_:20, 12;
offset,_:11,0;
}

impl Sv48Paddr {
    fn ppn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.ppn0()),
            1 => Some(self.ppn1()),
            2 => Some(self.ppn2()),
            3 => Some(self.ppn3()),
            _ => None
        }
    }
}

bitfield! {
struct Sv48Pte(RegT);
impl Debug;
ppn3,_:53, 37;
ppn2,_:36, 28;
ppn1,_:27, 19;
ppn0,_:18, 10;
rsw,_:9,8;
attr_raw,_:7,0;
}

impl Sv48Pte {
    fn ppn(&self, level: usize) -> Option<RegT> {
        match level {
            0 => Some(self.ppn0()),
            1 => Some(self.ppn1()),
            2 => Some(self.ppn2()),
            3 => Some(self.ppn3()),
            _ => None
        }
    }
}


macro_rules! pt_export {
    ($name:ident, $method:ident, $rt:ty, $($args:ident : $ty:ty),*) => {
        fn $method(&self, $($args : $ty,)*) -> $rt {
            match self {
                $name::Sv32(addr) => addr.$method($($args),*),
                $name::Sv39(addr)  => addr.$method($($args),*),
                $name::Sv48(addr) => addr.$method($($args),*),
            }
        }
    };
    ($name:ident, $method:ident, $rt:ty) => {
        fn $method(&self) -> $rt {
            match self {
                $name::Sv32(addr) => addr.$method(),
                $name::Sv39(addr)  => addr.$method(),
                $name::Sv48(addr) => addr.$method(),
            }
        }
    };
}

enum Vaddr {
    Sv32(Sv32Vaddr),
    Sv39(Sv39Vaddr),
    Sv48(Sv48Vaddr),
}

impl Vaddr {
    fn new(mode: PteMode) -> Option<Vaddr> {
        match mode {
            PteMode::Sv32 => Some(Vaddr::Sv32(Sv32Vaddr(0))),
            PteMode::Sv39 => Some(Vaddr::Sv39(Sv39Vaddr(0))),
            PteMode::Sv48 => Some(Vaddr::Sv48(Sv48Vaddr(0))),
            _ => None
        }
    }
    pt_export!(Vaddr, offset, RegT);
    pt_export!(Vaddr, vpn, Option<RegT>, level:usize);
}

enum Paddr {
    Sv32(Sv32Paddr),
    Sv39(Sv39Paddr),
    Sv48(Sv48Paddr),
}

impl Paddr {
    fn new(mode: PteMode) -> Option<Paddr> {
        match mode {
            PteMode::Sv32 => Some(Paddr::Sv32(Sv32Paddr(0))),
            PteMode::Sv39 => Some(Paddr::Sv39(Sv39Paddr(0))),
            PteMode::Sv48 => Some(Paddr::Sv48(Sv48Paddr(0))),
            _ => None
        }
    }
    pt_export!(Paddr, offset, RegT);
    pt_export!(Paddr, ppn, Option<RegT>, level:usize);
}

enum Pte {
    Sv32(Sv32Pte),
    Sv39(Sv39Pte),
    Sv48(Sv48Pte),
}

impl Pte {
    fn new(mode: PteMode) -> Option<Pte> {
        match mode {
            PteMode::Sv32 => Some(Pte::Sv32(Sv32Pte(0))),
            PteMode::Sv39 => Some(Pte::Sv39(Sv39Pte(0))),
            PteMode::Sv48 => Some(Pte::Sv48(Sv48Pte(0))),
            _ => None
        }
    }
    pt_export!(Pte, rsw, RegT);
    pt_export!(Pte, ppn, Option<RegT>, level:usize);
    pt_export!(Pte, attr_raw, RegT);
    fn attr(&self) -> PteAttr {
        (self.attr_raw() as u8).into()
    }
}


#[test]
fn pmp_basic_test() {
    let space = Arc::new(Space::new());
    let mut p = Processor::new(XLen::X32, &space);
    //no valid region
    assert_eq!(p.mmu().match_pmpcfg_entry(0, |p, entry| { true }), None);
    //NA4
    p.basic_csr.pmpcfg0.set_bit_range(4, 3, PmpAType::NA4.into());
    p.basic_csr.pmpaddr0.set(0x8000_0000 >> 2);
    assert!(p.mmu().match_pmpcfg_entry(0x8000_0000, |p, entry| { true }).is_some());
    //NAPOT
    p.basic_csr.pmpcfg3.set_bit_range(4, 3, PmpAType::NAPOT.into());
    p.basic_csr.pmpaddr12.set((0x2000_0000 + 0x1_0000 - 1) >> 2);
    assert!(p.mmu().match_pmpcfg_entry(0x2000_0000, |p, entry| { true }).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x2000_ffff, |p, entry| { true }).is_some());
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2000_ffff, |p, entry| { true }), p.mmu().match_pmpcfg_entry(0x2000_0000, |p, entry| { true }));
    assert_eq!(p.mmu().match_pmpcfg_entry(0x1000_ffff, |p, entry| { true }), None);
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2001_0000, |p, entry| { true }), None);
    //TOR
    p.basic_csr.pmpcfg3.set_bit_range(12, 11, PmpAType::TOR.into());
    p.basic_csr.pmpaddr13.set((0x2000_0000 + 0x1_0000) >> 2);
    p.basic_csr.pmpcfg3.set_bit_range(20, 19, PmpAType::TOR.into());
    p.basic_csr.pmpaddr14.set((0x2000_0000 + 0x2_0000) >> 2);
    assert!(p.mmu().match_pmpcfg_entry(0x2001_0000, |p, entry| { true }).is_some());
    assert!(p.mmu().match_pmpcfg_entry(0x2001_ffff, |p, entry| { true }).is_some());
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2002_0000, |p, entry| { true }), None);
    assert_eq!(p.mmu().match_pmpcfg_entry(0x2001_0000, |p, entry| { entry.l() == 1 }), None);
    p.basic_csr.pmpcfg3.set_bit_range(23, 23, 1);
    assert!(p.mmu().match_pmpcfg_entry(0x2001_0000, |p, entry| { entry.l() == 1 }).is_some());
}