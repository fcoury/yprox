/// Prints a hex dump of the given data with an optional direction string.
///
/// # Arguments
///
/// * `data` - A slice of bytes to be printed as a hex dump.
/// * `direction` - An optional string indicating the direction of the data flow.
///
/// # Example
///
/// ```
/// let data = [0x48, 0x65, 0x6C, 0x6C, 0x6F];
/// hex_dump(&data, "OUTGOING");
/// ```
pub fn hex_dump(data: &[u8], direction: &str) {
    const WIDTH: usize = 16;

    for chunk in data.chunks(WIDTH) {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..=0x7e).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        println!("{}: {:47}  |{}|", direction, hex.join(" "), ascii);
    }
}
