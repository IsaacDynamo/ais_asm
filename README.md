# VIA C3 Alternative Instruction Set Assembler

## Introduction
In 2018 [xoreaxeaxeax](https://twitter.com/xoreaxeaxeax) gave the talk [GOD MODE UNLOCKED - Hardware Backdoors in x86 CPUs](https://www.youtube.com/watch?v=_eSAF_qT_FY). In the talk he explains how he found undocumented instructions in VIA C3 processors.

Recently some leaked confidential documents can be found on the internet, describing parts of the AIS. The [VIA C3 Processor Alternative Instruction Set Application Note](http://www.bitsavers.org/components/viaTechnologies/C3-ais-appnote.pdf) and [VIA C3 Processor Alternative Instruction Set Programming Reference](http://www.bitsavers.org/components/viaTechnologies/C3-ais-reference.pdf). Based on these documents an assembler for the VIA C3 Alternative Instruction Set has been created.

## Project
The project contains two Rust packages, `ais_asm` and `kernel`.

The `ais_asm` is the Alternative Instruction Set Assembler. It doesn't parse an input file, but it is dynamic assembler. A program is created with Rust code and calls into the assembler. The `ais_asm/examples` folder contains some example programs.

The `kernel` is a mostly copied for an previous project, and is changed to contain and start the assembled payload. It is minimal kernel that can be run on VIA C3 hardware. And has a multiboot2 header and can be loaded with GRUB onto a target system. When the kernel is loaded it will initialize as serial port for `println!()` messages. Then try to enable AIS, and panic if the target doesn't support AIS. The kernel image includes a copy of the assembled program, and it will run this payload.

## Extra info
This project started as a submission for [LowLevelJam](https://github.com/LowLevelJam/LLJam0001). The demonstration can be found [here](low_level_jam.md).

Xoreaxeaxeax notes on AIS can by found in the [rosenbridge](https://github.com/xoreaxeaxeax/rosenbridge) repo.

The dynamic assembler design is from the youtube series [Bitwise](https://www.youtube.com/user/pervognsen), where one of the projects is a [RISCV assembler](https://github.com/pervognsen/bitwise/tree/master/ion/riscv).
