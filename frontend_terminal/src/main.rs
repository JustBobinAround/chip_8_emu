use std::{collections::{HashMap, HashSet}, io::{self, Read, Stdout, Write}, time::Duration};
use chip_8_core::{Emu, SCREEN_WIDTH, SCREEN_HEIGHT};

use crossterm::{
    cursor::{self, position}, event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode}, execute, style::{self, Stylize}, terminal::{disable_raw_mode, enable_raw_mode, Clear, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, QueueableCommand
};

fn reconstruct_op(d1: u8, d2: u8, d3: u8, d4: u8) -> u16 {
    let d1 = d1 as u16;
    let d2 = d2 as u16;
    let d3 = d3 as u16;
    let d4 = d4 as u16;
    (d1 << 12) | (d2 << 8) | (d3 << 4) | d4
}

fn print_events() -> io::Result<String> {
    let mut stdout = io::stdout();
    stdout.queue(EnterAlternateScreen)?;
    let mut emu = Emu::new();
    emu.load_rom(&"~/Downloads/octojam1title.ch8".to_string());
    let mut pixel_to_update: HashMap<(u16,u16), bool> = HashMap::new();
    loop {
        // Wait up to 1s for another event
        if poll(Duration::from_millis(16))? {
            // It's guaranteed that read() won't block if `poll` returns `Ok(true)`
            let event = read()?;

            if event == Event::Key(KeyCode::Char('q').into()) {
                break;
            }
        } else {
            refresh(&mut stdout, &mut emu, &mut pixel_to_update);
        }
    }
    stdout.queue(LeaveAlternateScreen)?;

    Ok(emu.debug)
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;

    match print_events() {
        Err(e) => {
            execute!(stdout, DisableMouseCapture)?;
            disable_raw_mode();
            println!("Error: {:?}\r", e);
        }
        Ok(debug) => {
            execute!(stdout, DisableMouseCapture)?;
            disable_raw_mode();
            println!("{}", debug);
        }

    }

    Ok(())

}

fn encode(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

fn decode(index: usize, width: usize) -> (usize, usize) {
    let x = index % width;
    let y = index / width;
    (x, y)
}

fn refresh(stdout: &mut Stdout, emu: &mut Emu, pixel_to_update: &mut HashMap<(u16, u16), bool>) -> io::Result<()>{
    emu.tick();
    for y in 0..SCREEN_HEIGHT as u16 {
        for x in 0..SCREEN_WIDTH as u16 {
            let pixel_state = emu.screen[encode(x as usize, y as usize, SCREEN_WIDTH)];

            if !pixel_to_update.get(&(x,y)).is_some_and(|p| *p==pixel_state) {
                if pixel_state {
                    stdout
                        .queue(cursor::MoveTo(x,y))?
                        .queue(style::PrintStyledContent("█".white()))?;
                } else {
                    stdout
                        .queue(cursor::MoveTo(x,y))?
                        .queue(style::PrintStyledContent("█".black()))?;
                }
                pixel_to_update.insert((x,y), pixel_state);
            }
            
        }
    }
    stdout.flush()?;

    Ok(())
}

