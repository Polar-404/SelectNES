#[cfg(feature = "debug_log")]
use std::io::BufRead;

#[cfg(feature = "debug_log")]
use selectnes::engine::instance::EmulatorInstance;

#[cfg(feature = "debug_log")]
use selectnes::debug::cpu_debug::log_state_nestest;

#[test]
fn test_nestest_execution() {
    #[cfg(feature = "debug_log")]
    {
        use selectnes::apu::audio::AudioOutput;

        let mut emulator = EmulatorInstance::new(std::path::PathBuf::from("tests/nestest/nestest.nes"))
            .unwrap_or_else(|_| panic!("couldn't start the emulator, nestest.nes might have been moved"));
        let mut audio  = AudioOutput::new();
        
        let mut logger = log_state_nestest(
            Some("tests/nestest_output.log"), 
            10_000,
        );

        emulator.cpu.reset_interrupt();
        emulator.cpu.program_counter = 0xC000;

        for _ in 0..=10_000 {
            emulator.run_frame_with_debug_logger(&mut audio, &mut logger);
            if emulator.cpu.last_opcode == 0x02 {
                println!("nestest stoped at a invalid opcode! {:04X}", emulator.cpu.program_counter);
                break
            }

            if emulator.cpu.program_counter == 0xC66E {
                println!("nestest successfully concluded.");
                break;
            }
        }

        if let Ok(logs) = selectnes::engine::terminal::struct_terminal::TERMINAL.lock() {
            for log in logs.iter() {
                println!("{:?}", log.log_msg);
            }
        } else {
            //this means something probably went wrong with the terminal
            //but I think there's no need to do anything here
        }

        //forces saving before the end of the program by dropping the variable
        std::mem::drop(logger);

        compare_files();
    }

    #[cfg(not(feature = "debug_log"))]
    {
        panic!("this test needs to be compiled with feature 'debug_log'");
    }
}


///compares onyl the pc and registers, ignores the remainder
#[cfg(feature = "debug_log")]
fn compare_files() {
    let output_file = std::fs::File::open("tests/nestest_output.log")
        .expect("Couldn't open the generated log");
    let expected_file = std::fs::File::open("tests/nestest/nestest.log")
        .expect("Couldn't open the official log");

    let output_lines = std::io::BufReader::new(output_file).lines();
    let expected_lines = std::io::BufReader::new(expected_file).lines();

    for (i, (actual_res, expected_res)) in output_lines.zip(expected_lines).enumerate() {
        let actual = actual_res.unwrap();
        let expected = expected_res.unwrap();

        let expected_pc = &expected[0..4];
        
        let expected_regs = &expected[48..73]; 
        
        let expected_clean = format!("{} {}", expected_pc, expected_regs);

        if actual != expected_clean {
            panic!(
                "\nFailed at line {}!\nExpected: {}\nGot:   {}\n", 
                i + 1, expected_clean, actual
            );
        }
    }
}