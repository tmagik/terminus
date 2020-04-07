use xmas_elf::ElfFile;
use xmas_elf::program::SegmentData;
use xmas_elf::header;
use xmas_elf::sections::SectionHeader;
use std::{fs, io};

pub struct ElfLoader {
    content: Box<[u8]>
}

impl ElfLoader {
    pub fn new(file: &str) -> io::Result<ElfLoader> {
        let content = fs::read(file)?.into_boxed_slice();
        Ok(ElfLoader {
            content
        })
    }

    fn elf(&self) -> Result<ElfFile<'_>, String> {
        match ElfFile::new(&self.content) {
            Ok(elf) => Ok(elf),
            Err(e) => Err(e.to_string())
        }
    }

    fn check_header(&self) -> Result<(), String> {
        let elf = self.elf()?;
        //check riscv
        if let header::Machine::Other(id) = elf.header.pt2.machine().as_machine() {
            if id == 243 {
                Ok(())
            } else {
                Err(format!("Invalid Arch {:?}!", elf.header.pt2.machine()))
            }
        } else {
            Err(format!("Invalid Arch {:?}!", elf.header.pt2.machine()))
        }
    }

    pub fn htif_section(&self) -> Result<Option<SectionHeader>, String> {
        let elf = self.elf()?;
        if let Some(s) = elf.find_section_by_name(".tohost") {
            Ok(Some(s))
        } else if let Some(s) = elf.find_section_by_name(".htif") {
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    pub fn entry_point(&self) -> Result<u64, String> {
        Ok(self.elf()?.header.pt2.entry_point())
    }

    pub fn load<F: Fn(u64, &[u8]) -> Result<(), String>>(&self, f: F) -> Result<(), String> {
        self.check_header()?;
        let elf = self.elf()?;
        let result = elf.program_iter().map(|p| {
            let data = match p.get_data(&elf)? {
                SegmentData::Undefined(d) => Ok(d),
                _ => Err("Only support Undefined SectionData for now!")
            };
            f(p.virtual_addr(), data?)
        });
        for r in result {
            if let Err(e) = r {
                return Err(e);
            }
        }
        Ok(())
    }
}