use terminus_global::{XLen, RegT};
use crate::processor::mmu::Mmu;
use std::marker::PhantomData;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use terminus_macros::*;
use crate::processor::extensions::i::csrs::*;
use std::rc::Rc;

#[derive(IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u8)]
pub enum PmpAType {
    OFF = 0,
    TOR = 1,
    NA4 = 2,
    NAPOT = 3,
}

bitfield! {
#[derive(Eq,PartialEq)]
pub struct PmpCfgEntry(u8);
impl Debug;
pub r, set_r:0, 0;
pub w, set_w:1, 1;
pub x, set_x:2, 2;
pub a, set_a:4,3;
pub l, set_l:7, 7;
}

impl From<u8> for PmpCfgEntry {
    fn from(v: u8) -> Self {
        PmpCfgEntry(v)
    }
}

pub struct PmpCfgsIter<'m> {
    icsrs: &'m Rc<ICsrs>,
    idx: u8,
    marker: PhantomData<&'m Mmu>,
}


impl<'m> PmpCfgsIter<'m> {
    pub fn new(icsrs: &'m Rc<ICsrs>, marker: PhantomData<&'m Mmu>) -> PmpCfgsIter<'m> {
        PmpCfgsIter {
            icsrs,
            idx: 0,
            marker,
        }
    }
    fn get_cfg(&self) -> RegT {
        match self.icsrs.xlen {
            XLen::X32 => {
                match (self.idx >> 2) & 0x3 {
                    0 => self.icsrs.pmpcfg0().get(),
                    1 => self.icsrs.pmpcfg1().get(),
                    2 => self.icsrs.pmpcfg2().get(),
                    3 => self.icsrs.pmpcfg3().get(),
                    _ => unreachable!()
                }
            }
            XLen::X64 => {
                match (self.idx >> 3) & 0x1 {
                    0 => self.icsrs.pmpcfg0().get(),
                    1 => self.icsrs.pmpcfg2().get(),
                    _ => unreachable!()
                }
            }
        }
    }

    fn get_entry(&self) -> PmpCfgEntry {
        let offset: u8 = match self.icsrs.xlen {
            XLen::X32 => self.idx & 0x3,
            XLen::X64 => self.idx & 0x7,
        };
        let cfg:u8 = (self.get_cfg() >> ((offset as RegT) << 3)) as u8;

        cfg.into()
    }
}

impl<'m> Iterator for PmpCfgsIter<'m> {
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