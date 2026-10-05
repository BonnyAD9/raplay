use std::io::stdin;

use flexi_logger::Logger;
use raplay::{Sink, source::Symph};

fn main() -> raplay::Result<()> {
    let _l = Logger::try_with_env_or_str("info")
        .unwrap()
        .start()
        .unwrap();

    // Get the path to the audio file.
    let home = std::env::home_dir().unwrap_or_else(|| ".".into());
    let path = home
        .join("music/Jacob Collier - Djesse Vol. 4/01. 100,000 Voices.flac");

    // Use symphonia decoder for the file.
    let src = Symph::open(path, &Default::default())?;

    // Initialize the sink.
    let mut sink = Sink::new();
    sink.volume(0.3)?; // Set the volume to some reasonable value.

    // Log any errors.
    sink.on_err_callback(Box::new(|e| eprintln!("Error: {e}")))?;

    // Play the symphonia source.
    sink.load(Box::new(src), true)?;

    loop {
        // Wait for enter.
        _ = stdin().read_line(&mut String::new());

        // Toggle play/pause.
        sink.play(!sink.is_playing()?)?;

        // Print the current timestamp.
        let ts = sink.get_timestamp()?;
        println!("{:?}/{:?}", ts.current, ts.total);
    }
}
