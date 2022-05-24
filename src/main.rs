use image::io::Reader;
use image::*;
use imgtotext::*;
use rusttype::*;

const UBUNTU_MONO: &str = "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf";
const DAD: &str = "dad_yoism_la_times.jpg";

fn main() -> std::io::Result<()> {
  let font_path =
    if let Some(path) = std::env::args().nth(1) { path } else { UBUNTU_MONO.to_string() };
  let font = Font::try_from_vec(std::fs::read(font_path)?).unwrap();

  print!("{}", render(Reader::open(DAD)?.decode().unwrap().to_luma8(), font, 60, 50.));

  Ok(())
}
