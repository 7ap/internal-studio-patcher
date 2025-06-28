use goblin::mach::{Mach, MachO};
use iced_x86::{Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register};
use std::path::PathBuf;
use std::sync::Mutex;

static INSTRUCTIONS: Mutex<Vec<Instruction>> = Mutex::new(Vec::new()); // i STILL hate the upper snake case but i also hate warnings

fn find_random_strings_that_give_us_internal_studio(
    pe: &MachO,
    input: &[u8],
    target: &[u8],
) -> Option<u64> {
    for seg in &pe.segments {
        if seg.name().unwrap_or_default() != "__TEXT" {
            continue;
        }
        let secs = seg.sections().ok()?;
        for (sect, _) in secs {
            if sect.segname().unwrap_or_default() == "__TEXT"
                && std::str::from_utf8(&sect.sectname)
                    .unwrap_or_default()
                    .trim_end_matches('\0')
                    == "__cstring"
            {
                let start = sect.offset as usize;
                let size = sect.size as usize;
                if let Some(off) = input[start..start + size]
                    .windows(target.len())
                    .position(|w| w == target)
                {
                    return Some(sect.addr + off as u64);
                }
            }
        }
    }
    None
}

fn get_jz_that_controls_internal_studio(
    voicechat_addr: u64,
    instructions: &[Instruction],
) -> Option<u64> {
    let start_idx = instructions
        .iter()
        .position(|insn| insn.ip() == voicechat_addr)
        .unwrap();
    let mut identifier_function_addr = None;
    for idx in (0..=start_idx).rev() {
        let insn = &instructions[idx];
        if insn.mnemonic() == Mnemonic::Call {
            identifier_function_addr = Some(insn.near_branch_target());
            break;
        }
    }
    if let Some(target_addr) = identifier_function_addr {
        for (i, insn) in instructions.iter().enumerate() {
            if insn.mnemonic() == Mnemonic::Call && insn.near_branch_target() == target_addr {
                let prev = &instructions[i - 1];
                if prev.mnemonic() == Mnemonic::Je {
                    return Some(prev.ip());
                }
            }
        }
    }
    None
}

pub fn start(mut input: Vec<u8>, _output: &PathBuf) {
    let macho = match Mach::parse(&input).unwrap() {
        Mach::Binary(m) => m,
        _ => panic!("Error: Could not parse Roblox binary. Please report to https://github.com/7ap/internal-studio-patcher/issues"),
    };
    let voicechat_addr = find_random_strings_that_give_us_internal_studio(
        &macho,
        &input,
        b"VoiceChatEnableApiSecurityCheck",
    )
    .expect("Error: Could not find the first string that is searched for to get internal studio. Please report to https://github.com/7ap/internal-studio-patcher/issues");
    let start_api_addr = find_random_strings_that_give_us_internal_studio(
        &macho,
        &input,
        b"Start API Dump",
    )
    .expect("Error: Could not find the second string that is searched for to get internal studio. Please report to https://github.com/7ap/internal-studio-patcher/issues");
    let (text_o, text_s, text_b) = macho
        .segments
        .iter()
        .filter_map(|seg| {
            if seg.name().unwrap_or_default() == "__TEXT" {
                seg.sections().ok()?.into_iter().find_map(|(sect, _)| {
                    let name = std::str::from_utf8(&sect.sectname)
                        .unwrap_or_default()
                        .trim_end_matches('\0');
                    if sect.segname().unwrap_or_default() == "__TEXT" && name == "__text" {
                        Some((sect.offset as usize, sect.size as usize, sect.addr))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .next()
        .expect(
            "text missing (this error isnt formatted because this literally should never happen)",
        );
    let text_bytes = &input[text_o..text_o + text_s];
    let mut dec = Decoder::with_ip(64, text_bytes, text_b, DecoderOptions::NONE);
    let mut insn = Instruction::default();
    while dec.can_decode() {
        dec.decode_out(&mut insn);
        INSTRUCTIONS.lock().unwrap().push(insn);
    }
    let instrs = INSTRUCTIONS.lock().unwrap();
    let start_idx = instrs
        .iter()
        .position(|i| i.is_ip_rel_memory_operand() && i.ip_rel_memory_address() == start_api_addr)
        .unwrap();
    let func_start = (0..=start_idx)
        .rev()
        .find(|&i| {
            let i = &instrs[i];
            i.mnemonic() == Mnemonic::Push
                && i.op_count()   == 1
                && i.op0_kind()   == OpKind::Register
                && i.op0_register() == Register::RBP
        })
        .expect("this error also should never happen, report to https://github.com/7ap/internal-studio-patcher/issues if you get it");
    let mut thinger = 0;
    for i in func_start..=start_idx {
        let i = &instrs[i];
        if i.is_ip_rel_memory_operand() && i.ip_rel_memory_address() == voicechat_addr {
            thinger = i.ip();
        }
    }
    let patch_me = get_jz_that_controls_internal_studio(thinger, &instrs);
    if let Some(patch_me) = patch_me {
        let raw_start = text_o;
        let text_start = text_b;
        let offset = raw_start + (patch_me - text_start) as usize;
        let len = instrs
            .iter()
            .find(|i| i.ip() == patch_me)
            .map(|i| i.len())
            .unwrap_or(0);
        input[offset..offset + len].fill(0x90);
        std::fs::write(_output, &input).unwrap();
    } else {
        eprintln!("Error: Could not find the address to patch (have you already patched studio?). Please report to https://github.com/7ap/internal-studio-patcher/issues");
        std::process::exit(1);
    }
}
