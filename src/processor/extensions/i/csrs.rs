use crate::prelude::*;

csr_map! {
pub ICsrs(0x0, 0xfff) {
    cycle(RO):Cycle, 0xC00;
    instret(RO):Instret, 0xC02;
    cycleh(RO):Cycle, 0xC80;
    instreth(RO):Instret, 0xC82;
    mstatus(RW):MStatus, 0x300;
    misa(RW):Misa, 0x301;
    medeleg(RW):Medeleg, 0x302;
    mideleg(RW):Mideleg, 0x303;
    mie(RW):Mie, 0x304;
    mtvec(RW):Tvec, 0x305;
    mcounteren(RW):Counteren, 0x306;
    mscratch(RW):Scratch, 0x340;
    mepc(RW):Epc, 0x341;
    mcause(RW):Cause, 0x342;
    mtval(RW):Tval, 0x343;
    mip(RW):Mip, 0x344;
    pmpcfg0(RW):PmpCfg, 0x3A0;
    pmpcfg1(RW):PmpCfg, 0x3A1;
    pmpcfg2(RW):PmpCfg, 0x3A2;
    pmpcfg3(RW):PmpCfg, 0x3A3;
    pmpaddr0(RW):PmpAddr, 0x3B0;
    pmpaddr1(RW):PmpAddr, 0x3B1;
    pmpaddr2(RW):PmpAddr, 0x3B2;
    pmpaddr3(RW):PmpAddr, 0x3B3;
    pmpaddr4(RW):PmpAddr, 0x3B4;
    pmpaddr5(RW):PmpAddr, 0x3B5;
    pmpaddr6(RW):PmpAddr, 0x3B6;
    pmpaddr7(RW):PmpAddr, 0x3B7;
    pmpaddr8(RW):PmpAddr, 0x3B8;
    pmpaddr9(RW):PmpAddr, 0x3B9;
    pmpaddr10(RW):PmpAddr, 0x3BA;
    pmpaddr11(RW):PmpAddr, 0x3BB;
    pmpaddr12(RW):PmpAddr, 0x3BC;
    pmpaddr13(RW):PmpAddr, 0x3BD;
    pmpaddr14(RW):PmpAddr, 0x3BE;
    pmpaddr15(RW):PmpAddr, 0x3BF;
    //no debug
    tselect(RO):Tselect, 0x7A0;
    mcycle(RO):Cycle, 0xB00;
    minstret(RO):Instret, 0xB02;
    mcycleh(RO):Cycle, 0xB80;
    minstreth(RO):Instret, 0xB82;
    mvendorid(RO):Mvendorid, 0xF11;
    marchid(RO):Marchid, 0xF12;
    mimpid(RO):Mimpid, 0xF13;
    mhartid(RO):Mhartid, 0xF14;
}
}

define_csr! {
Mvendorid {
}
}

define_csr! {
Marchid {
}
}

define_csr! {
Mimpid {
}
}


define_csr! {
Misa {
    fields {
        a(RW):0,0;
        b(RO):1,1;
        c(RW):2,2;
        d(RW):3,3;
        e(RO):4,4;
        f(RW):5,5;
        g(RO):6,6;
        h(RO):7,7;
        i(RO):8,8;
        j(RO):9,9;
        k(RO):10,10;
        l(RO):11,11;
        m(RW):12,12;
        n(RO):13,13;
        o(RO):14,14;
        p(RO):15,15;
        q(RO):16,16;
        r(RO):17,17;
        s(RO):18,18;
        t(RO):19,19;
        u(RO):20,20;
        v(RO):21,21;
        w(RO):22,22;
        x(RO):23,23;
        y(RO):24,24;
        z(RO):25,25;
    },
    fields32{
        mxl(RO):31,30;
    },
    fields64{
        mxl(RO):63,62;
    },
}
}

define_csr! {
MStatus {
    fields {
         uie(RW): 0, 0;
         sie(RW): 1, 1;
         mie(RW): 3, 3;
         upie(RW): 4, 4;
         spie(RW): 5, 5;
         mpie(RW): 7, 7;
         spp(RW): 8, 8;
         mpp(RW): 12, 11;
         fs(RW): 14, 13;
         xs(RW): 16, 15;
         mprv(RW): 17, 17;
         sum(RW): 18, 18;
         mxr(RW): 19, 19;
         tvm(RW): 20, 20;
         tw(RW): 21, 21;
         tsr(RW): 22, 22;
    },
    fields32 {
         sd(RW): 31, 31;
    },
    fields64 {
         uxl(RO): 33, 32;
         sxl(RO): 35,34;
         sd(RW): 63, 63;
    },
}
}

define_csr! {
PmpCfg{
    fields {
        pmpcfg0(RW):7,0;
        pmpcfg1(RW):15,8;
        pmpcfg2(RW):23,16;
        pmpcfg3(RW):31,24;
    },
    fields64{
        pmpcfg4(RW):39,32;
        pmpcfg5(RW):47,40;
        pmpcfg6(RW):55,48;
        pmpcfg7(RW):63,56;
    }
}
}

impl BitRange<u8> for PmpCfg {
    fn bit_range(&self, msb: usize, lsb: usize) -> u8 {
        let width = msb - lsb + 1;
        let mask = (1 << width) - 1;
        ((self.get() >> lsb) & mask) as u8
    }
    fn set_bit_range(&mut self, msb: usize, lsb: usize, value: u8) {
        let width = msb - lsb + 1;
        let mask = !((((1 << width) - 1) << lsb) as RegT);
        self.set((value as RegT) << (lsb as RegT) | self.get() & mask)
    }
}

define_csr! {
PmpAddr {
    fields32 {
        addr(RW):31, 0;
    },
    fields64 {
        addr(RW):53, 0;
    }
}
}

define_csr! {
Mhartid{}
}

define_csr! {
Tvec {
    fields{
        mode(RW):0,0;
    },
    fields32{
        base(RW):31, 2;
    },
    fields64{
        base(RW):63, 2;
    }
}
}

define_csr! {
Medeleg{}
}

define_csr! {
Mideleg {
    fields{
        usip(RW):0,0;
        ssip(RW):1,1;
        msip(RW):3,3;
        utip(RW):4,4;
        stip(RW):5,5;
        mtip(RW):7,7;
        ueip(RW):8,8;
        seip(RW):9,9;
        meip(RW):11,11;
    }
}
}

define_csr! {
Mip {
    fields{
        usip(RW):0,0;
        ssip(RW):1,1;
        msip(RO):3,3;
        utip(RW):4,4;
        stip(RW):5,5;
        mtip(RO):7,7;
        ueip(RW):8,8;
        seip(RW):9,9;
        meip(RO):11,11;
    }
}
}

define_csr! {
Mie {
    fields{
        usie(RW):0,0;
        ssie(RW):1,1;
        msie(RW):3,3;
        utie(RW):4,4;
        stie(RW):5,5;
        mtie(RW):7,7;
        ueie(RW):8,8;
        seie(RW):9,9;
        meie(RW):11,11;
    }
}
}

define_csr! {
Epc {}
}

define_csr! {
Cause {
    fields {
       code(RW):3,0;
    },
    fields32{
        int(RW):31,31;
    },
    fields64{
        int(RW):63,63;
    }
}
}

define_csr! {
Tval {}
}

define_csr! {
Scratch {}
}

define_csr! {
Tselect {}
}

define_csr! {
Counteren {
    fields {
       cy(RW):0, 0;
       ir(RW):2, 2;
    },
}
}

define_csr! {
Cycle {
    fields32 {
       cycle(RO):31, 0;
    },
    fields64 {
       cycle(RO):63, 0;
    },
}
}

define_csr! {
Instret {
    fields32 {
       instret(RO):31, 0;
    },
    fields64 {
       instret(RO):63, 0;
    },
}
}

#[test]
fn test_status() {
    let mut status = MStatus::new(XLen::X32, 0);
    status.set_xs(0xf);
    assert_eq!(status.xs(), 0x3);
    status.set_xs(0);
    assert_eq!(status.xs(), 0);
}

