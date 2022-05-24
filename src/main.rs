use clap::Parser;
use fontconfig::Fontconfig;
use image::io::Reader;
use image::*;
use imgtotext::*;
use rusttype::*;
use tap::prelude::*;

#[derive(Parser)]
#[clap(author, about)]
struct Args {
  /// Font family to pass to Fontconfig.
  #[clap(short, default_value = "mono")]
  font: String,

  /// The origin for the colorspace of the image. Can also be thought of a
  /// darkness penalty.
  #[clap(short, default_value = "10")]
  penalty: f32,

  /// The width of the resulting image in characters.
  #[clap(short, default_value = "80")]
  w_img_char: u32,

  /// Draw the picture normally, without applying an edge detection filter.
  #[clap(short)]
  normal: bool,

  /// The path to the image to use.
  img: String,
}

fn main() -> std::io::Result<()> {
  let args = Args::parse();

  let font_path = Fontconfig::new()
    .expect("Couldn't link to Font Config.")
    .find(&args.font, None)
    .expect("Could not find a matching font.")
    .tap(|it| println!("Using font {} at {:?}", it.name, it.path))
    .path;
  let font = Font::try_from_vec(std::fs::read(font_path)?).unwrap();

  print!(
    "{}",
    render(
      Reader::open(args.img)?.decode().unwrap().to_luma8(),
      font,
      args.w_img_char,
      args.penalty,
      !args.normal,
    )
  );

  Ok(())
}
