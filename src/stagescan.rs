use goblin::pe::PE;
use iced_x86::{Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register};
use std::path::PathBuf;
use std::sync::Mutex;

static INSTRUCTIONS: Mutex<Vec<Instruction>> = Mutex::new(Vec::new()); // i hate the upper snake case but i also hate warnings

fn find_this_random_string_that_lets_us_get_internal_studio(pe: &PE, input: &[u8]) -> Option<u64> {
    let voicechatstringthatisthekeytogettinginternalstudioforsomereason =
        b"VoiceChatEnableApiSecurityCheck";
    let mut string_addr = None;
    for sect in &pe.sections {
        let name = sect.name().unwrap_or_default();
        if name == ".rdata" || name == ".data" {
            let start = sect.pointer_to_raw_data as usize;
            let size = sect.size_of_raw_data as usize;
            if let Some(off) = input[start..start + size]
                .windows(voicechatstringthatisthekeytogettinginternalstudioforsomereason.len())
                .position(|w| w == voicechatstringthatisthekeytogettinginternalstudioforsomereason)
            {
                string_addr =
                    Some((pe.image_base as u64) + (sect.virtual_address as u64 + off as u64));
                break;
            }
        }
    }
    string_addr
}
fn get_jz_that_controls_internal_studio(random_string_internal_studio: u64) -> Option<u64> {
    let mut identifier_function = None;
    let instructions = INSTRUCTIONS.lock().unwrap();
    let mut patch_addr = None;

    for (i, insn) in instructions.iter().enumerate() {
        for op in 0..insn.op_count() {
            if insn.op_kind(op) != OpKind::Memory {
                continue;
            }
            if insn.memory_base() == Register::RIP {
                if insn.memory_displacement64() == random_string_internal_studio
                    && insn.mnemonic() != Mnemonic::Lea
                {
                    for back in (0..i).rev() {
                        let prev = &instructions[back];
                        if prev.mnemonic() == Mnemonic::Call {
                            if back > 0 && instructions[back - 1].mnemonic() == Mnemonic::Lea {
                                continue;
                            } else {
                                identifier_function = Some(prev.near_branch_target());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(identifier_function) = identifier_function {
        for (i, insn) in instructions.iter().enumerate() {
            if insn.mnemonic() == Mnemonic::Call && insn.near_branch_target() == identifier_function
            {
                let prev = instructions[i - 1];
                if prev.mnemonic() == Mnemonic::Je {
                    patch_addr = Some(prev.ip());
                    break;
                }
            }
        }
    } else {
        eprintln!("Error: Could not find the identifier function.  Please report to https://github.com/7ap/internal-studio-patcher/issues");
        std::process::exit(1);
    }
    patch_addr
}

pub fn start(mut input: Vec<u8>, output: &PathBuf) {
    let pe = PE::parse(&input).unwrap();
    let str_addr = find_this_random_string_that_lets_us_get_internal_studio(&pe, &input).expect("Error: Could not find the string that is searched for to get internal studio. Please report to https://github.com/7ap/internal-studio-patcher/issues");
    let text = pe
        .sections
        .iter()
        .find(|s| s.name().unwrap_or_default() == ".text")
        .expect(
            ".text missing (this error isnt formatted because this literally should never happen)",
        );
    let raw_start = text.pointer_to_raw_data as usize;
    let raw_size = text.size_of_raw_data as usize;
    let text_start = (pe.image_base as u64) + text.virtual_address as u64;
    let text_bytes = &input[raw_start..raw_start + raw_size];
    let mut dec = Decoder::with_ip(64, text_bytes, text_start, DecoderOptions::NONE);
    let mut instruction = Instruction::default();
    while dec.can_decode() {
        dec.decode_out(&mut instruction);
        INSTRUCTIONS.lock().unwrap().push(instruction);
    }
    let patch_me = get_jz_that_controls_internal_studio(str_addr);
    if let Some(patch_me) = patch_me {
        let offset = raw_start + (patch_me - text_start) as usize;
        let len = INSTRUCTIONS
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.ip() == patch_me)
            .map(|i| i.len())
            .unwrap_or(0);
        input[offset..offset + len].fill(0x90);
        std::fs::write(output, &input).unwrap();
    } else {
        eprintln!("Error: Could not find the address to patch. Please report to https://github.com/7ap/internal-studio-patcher/issues");
        std::process::exit(1);
    }
}
