use conv::ValueInto;
use float_ord::FloatOrd;
use image::buffer::ConvertBuffer;
use image::imageops::{resize, FilterType};
use image::*;
use imageproc::definitions::*;
use imageproc::filter::*;
use imageproc::gradients::*;
use imageproc::map::*;
use num::{Num, Signed};
use rusttype::*;
use tap::prelude::*;

pub type Gray32FImage = ImageBuffer<Luma<f32>, Vec<f32>>;

const _CP437: &str = r##"☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼ !"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~⌂ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■"##;

pub fn render(
  img: GrayImage,
  font: Font,
  w_img_char: u32,
  // penalty: f32,
  origin: f32,
  edge_filter: bool,
) -> String {
  let scale = Scale::uniform(20.);

  /* --- Prepare characters and calculate bounding box --- */

  let chars = (32..127).map(char::from_u32).flatten();
  // let chars = CP437.chars();
  let glyphs = chars
    .clone()
    .map(|c| font.glyph(c).scaled(scale).positioned(Point { x: 0., y: 0. }));

  let bb = glyphs.clone().fold(
    Rect { min: point(i32::MAX, i32::MAX), max: point(i32::MIN, i32::MIN) },
    |bb, g| match g.pixel_bounding_box() {
      Some(gb) => Rect {
        min: point(bb.min.x.min(gb.min.x), bb.min.y.min(gb.min.y)),
        max: point(bb.max.x.max(gb.max.x), bb.max.y.max(gb.max.y)),
      },
      None => bb,
    },
  );

  /* --- Calculate various dimensions --- */

  let w_char_px = font.glyph('m').scaled(scale).h_metrics().advance_width;
  let h_char_px =
    font.v_metrics(scale).pipe(|it| it.ascent - it.descent + it.line_gap);
  let h_img_char = (w_char_px / h_char_px
    * w_img_char as f32
    * img.dimensions().pipe(|it| it.1 as f32 / it.0 as f32))
    as u32;

  /* --- Preprocess --- */

  // let img: Gray32FImage = img.convert();

  let img = resize(
    &img,
    w_img_char * bb.width() as u32,
    h_img_char * bb.height() as u32,
    FilterType::Triangle,
  );

  let img = if edge_filter {
    let img = gaussian_blur_f32(&img, 1.4);
    let img = sobel_gradient_map(&img, |p| {
      p.channels()
        .pipe(<[u16; 1]>::try_from)
        .unwrap()
        .map(|it| it as f32 / 255. / 20f32.sqrt())
        .pipe(Luma)
    });
    let max =
      img.pixels().map(|p| p.channels()[0].pipe(FloatOrd)).max().unwrap().0;
    let img = map_subpixels(&img, |p| p / max * (1. - origin) + origin);
    img
  } else {
    let mut img: DynamicImage = img.into();
    img.invert();
    img.to_luma32f()
  };

  {
    let it: GrayImage = img.convert();
    it
  }
  .save("out/edges.png")
  .unwrap();

  /* --- Render characters --- */

  let kernels: Vec<_> = glyphs
    .clone()
    .map(|g| {
      let mut ker = vec![0.; bb.width() as usize * bb.height() as usize];
      if let Some(gb) = g.pixel_bounding_box() {
        g.draw(|x, y, v| {
          assert!(x < bb.width() as u32);
          assert!(y < bb.height() as u32);
          ker[(x as i32 + gb.min.x - bb.min.x) as usize
            + (y as i32 + gb.min.y - bb.min.y) as usize
              * bb.width() as usize] = v; //v + origin * (1. - v);
        });
      }

      // let norm = ker.iter().map(|it| it.powi(2)).sum::<f32>().sqrt().powi(2);
      // ker.iter_mut().for_each(|it| *it /= norm);
      OwnedKernel::new(ker, bb.width() as u32, bb.height() as u32)
    })
    .collect();

  /* --- Save characters for validation --- */

  for (c, ker) in chars.clone().zip(&kernels) {
    GrayImage::from_vec(
      bb.width() as u32,
      bb.height() as u32,
      ker.data.iter().map(|f| (f * 255.) as u8).collect(),
    )
    .unwrap()
    .save(format!("out/{}.png", c))
    .pipe(std::mem::drop);
  }

  // let penalties: Vec<_> =
  //   kernels.iter().map(|ker| ker.data.iter().sum::<f32>() * penalty).collect();

  /* --- Render best fit characters to a string --- */

  let mut out = String::new();
  for y in 0..h_img_char {
    for x in 0..w_img_char {
      out.push(
        chars
          .clone()
          .zip(kernels.iter())
          // .zip(penalties.iter())
          .min_by_key(|(_, ker)| {
            ker
              .cost(
                &img,
                bb.width() as u32 * x + bb.width() as u32 / 2,
                bb.height() as u32 * y + bb.height() as u32 / 2,
              )
              .pipe(FloatOrd)
          })
          .unwrap()
          .0,
      );
    }
    out.push('\n');
  }
  out
}

/// A wrapper around imageproc::filter::Kernel that owns it's data.
pub struct OwnedKernel<C, K> {
  data: C,
  width: u32,
  height: u32,
  _unused: std::marker::PhantomData<K>,
}

impl<'a, C: AsRef<[K]>, K: Num + Copy + 'a> OwnedKernel<C, K> {
  fn new(data: C, width: u32, height: u32) -> OwnedKernel<C, K> {
    Kernel::new(data.as_ref(), width, height); // Just to check assertions
    OwnedKernel::<C, K> {
      data,
      width,
      height,
      _unused: std::marker::PhantomData,
    }
  }

  pub fn filter<P, F, Q>(&self, image: &Image<P>, f: F) -> Image<Q>
  where
    P: Pixel,
    <P as Pixel>::Subpixel: ValueInto<K>,
    Q: Pixel,
    F: FnMut(&mut Q::Subpixel, K),
  {
    Kernel::new(self.data.as_ref(), self.width, self.height).filter(image, f)
  }

  /// Combine scalar values from the image and the kernel with f and add them
  /// up, with the kernel only in one position.
  /// Partly copied from imageproc::filter::Kernel::filter
  /// Note: currently just uses first channel and ignores others
  pub fn sample<P, F>(&self, image: &Image<P>, x: u32, y: u32, mut f: F) -> K
  where
    P: Pixel,
    <P as Pixel>::Subpixel: ValueInto<K>,
    F: FnMut(K, K) -> K,
  {
    let (width, height) = image.dimensions();
    let mut out = K::zero();
    let (k_width, k_height) = (self.width as i64, self.height as i64);
    let (width, height) = (width as i64, height as i64);

    for k_y in 0..k_height {
      let y_p = (height - 1).min(0.max(y as i64 + k_y - k_height / 2)) as u32;
      for k_x in 0..k_width {
        let x_p = (width - 1).min(0.max(x as i64 + k_x - k_width / 2)) as u32;
        out = out
          + f(
            unsafe { &image.unsafe_get_pixel(x_p, y_p) }.channels()[0]
              .value_into()
              .unwrap(),
            unsafe {
              *self.data.as_ref().get_unchecked((k_y * k_width + k_x) as usize)
            },
          );
      }
    }
    out
  }

  /// Distance squared from a cell of the image around a point
  /// Note: serious code duplication from Self::sample, could
  /// be abstracted but nah.
  pub fn cost<P>(&self, image: &Image<P>, x: u32, y: u32) -> K
  where
    P: Pixel,
    <P as Pixel>::Subpixel: ValueInto<K>,
    K: Signed,
  {
    let (width, height) = image.dimensions();
    let mut out = K::zero();
    let (k_width, k_height) = (self.width as i64, self.height as i64);
    let (width, height) = (width as i64, height as i64);

    for k_y in 0..k_height {
      let y_p = (height - 1).min(0.max(y as i64 + k_y - k_height / 2)) as u32;
      for k_x in 0..k_width {
        let x_p = (width - 1).min(0.max(x as i64 + k_x - k_width / 2)) as u32;
        out = out
          + (unsafe { &image.unsafe_get_pixel(x_p, y_p) }.channels()[0]
            .value_into()
            .unwrap()
            - unsafe {
              *self.data.as_ref().get_unchecked((k_y * k_width + k_x) as usize)
            })
          // .pipe(|it| it * it);
          .pipe(|it| it.abs());
      }
    }
    out
  }
}
