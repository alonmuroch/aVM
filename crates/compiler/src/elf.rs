use goblin::elf::Elf;
use goblin::elf::section_header::SHT_NOBITS;

pub struct ElfInfo<'a> {
    pub code: &'a [u8],
    pub sections: Vec<ElfSection<'a>>,
}

pub struct ElfSection<'a> {
    pub name: String,
    pub addr: u64,
    pub size: u64,
    pub data: &'a [u8],
}

impl<'a> ElfInfo<'a> {
    /// Returns a flat buffer with all `.text*` sections merged, and the base address.
    pub fn get_flat_code(&self) -> Option<(Vec<u8>, u64)> {
        let text_sections: Vec<&ElfSection> = self.sections
            .iter()
            .filter(|s| s.name.starts_with(".text"))
            .collect();

        if text_sections.is_empty() {
            return None;
        }

        let min_addr = text_sections.iter().map(|s| s.addr).min().unwrap();
        let max_addr = text_sections.iter().map(|s| s.addr + s.size).max().unwrap();

        let total_size = (max_addr - min_addr) as usize;
        let mut flat_code = vec![0u8; total_size];

        for section in text_sections {
            let offset = (section.addr - min_addr) as usize;
            flat_code[offset..offset + section.data.len()].copy_from_slice(section.data);
        }

        Some((flat_code, min_addr))
    }

    /// Returns a flat buffer with all `.rodata*` sections merged, and the base address.
    pub fn get_flat_rodata(&self) -> Option<(Vec<u8>, u64)> {
        let rodata_sections: Vec<&ElfSection> = self.sections
            .iter()
            .filter(|s| s.name.starts_with(".rodata"))
            .collect();

        if rodata_sections.is_empty() {
            return None;
        }

        let min_addr = rodata_sections.iter().map(|s| s.addr).min().unwrap();
        let max_addr = rodata_sections.iter().map(|s| s.addr + s.size).max().unwrap();

        let total_size = (max_addr - min_addr) as usize;
        let mut flat_rodata = vec![0u8; total_size];

        for section in rodata_sections {
            let offset = (section.addr - min_addr) as usize;
            flat_rodata[offset..offset + section.data.len()].copy_from_slice(section.data);
        }

        Some((flat_rodata, min_addr))
    }

    /// Returns a reference to a section by its name, if it exists.
    /// Only the first section with the given name is returned, or None if not found.
    pub fn get_section_by_name(&self, name: &str) -> Option<&ElfSection<'a>> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Returns a flat `.bss` range (length is zeroed by loader), and base address.
    pub fn get_flat_bss(&self) -> Option<(Vec<u8>, u64)> {
        let bss_sections: Vec<&ElfSection> = self
            .sections
            .iter()
            .filter(|s| s.name.starts_with(".bss") || s.name.starts_with(".sbss"))
            .collect();

        if bss_sections.is_empty() {
            return None;
        }

        let min_addr = bss_sections.iter().map(|s| s.addr).min().unwrap();
        let max_addr = bss_sections
            .iter()
            .map(|s| s.addr + s.size)
            .max()
            .unwrap();

        let total_size = (max_addr - min_addr) as usize;
        Some((vec![0u8; total_size], min_addr))
    }
}


pub fn parse_elf_from_bytes<'a>(bytes: &'a [u8]) -> Result<ElfInfo<'a>, goblin::error::Error> {
    let elf = Elf::parse(bytes)?;

    let mut sections = Vec::new();
    for section in elf.section_headers.iter() {
        if let Some(name) = elf.shdr_strtab.get_at(section.sh_name) {
            let offset = section.sh_offset as usize;
            let size = section.sh_size as usize;

            let is_nobits = section.sh_type == SHT_NOBITS;
            if offset + size <= bytes.len() || is_nobits {
                let data = if is_nobits {
                    &bytes[0..0]
                } else {
                    &bytes[offset..offset + size]
                };
                sections.push(ElfSection {
                    name: name.to_string(),
                    addr: section.sh_addr,
                    size: section.sh_size,
                    data,
                });
            }
        }
    }

    Ok(ElfInfo { code: bytes, sections })
}
