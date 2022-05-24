use conv::ValueInto;
use float_ord::FloatOrd;
use image::buffer::ConvertBuffer;
use image::*;
use imageproc::definitions::*;
use imageproc::filter::*;
use num::Num;
use rusttype::*;
use tap::prelude::*;

pub fn render(img: GrayImage, font: Font, w_img_char: u32, penalty: f32) -> String {
  let w_char_px = img.dimensions().0 as f32 / w_img_char as f32;
  let scale = w_char_px / font.glyph('m').scaled(Scale::uniform(1.)).h_metrics().advance_width;
  let scale = Scale::uniform(scale);
  let h_char_px = font.v_metrics(scale).pipe(|it| it.ascent - it.descent + it.line_gap);
  let h_img_char = (img.dimensions().1 as f32 / h_char_px).round() as u32;

  let chars = (32..127).map(char::from_u32).flatten();
  let glyphs =
    chars.clone().map(|c| font.glyph(c).scaled(scale).positioned(Point { x: 0., y: 0. }));

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
  println!("bb: {}..{}, {}..{}", bb.min.x, bb.max.x, bb.min.y, bb.max.x);

  let kernels: Vec<_> = glyphs
    .clone()
    .map(|g| {
      let mut ker = vec![0.; bb.width() as usize * bb.height() as usize];
      if let Some(gb) = g.pixel_bounding_box() {
        g.draw(|x, y, v| {
          assert!(x < bb.width() as u32);
          assert!(y < bb.height() as u32);
          ker[(x as i32 + gb.min.x - bb.min.x) as usize
            + (y as i32 + gb.min.y - bb.min.y) as usize * bb.width() as usize] = v;
        });
      }
      OwnedKernel::new(ker, bb.width() as u32, bb.height() as u32)
    })
    .collect();

  for (c, ker) in chars.clone().zip(&kernels) {
    GrayImage::from_vec(
      bb.width() as u32,
      bb.height() as u32,
      ker.data.iter().map(|f| (f * 255.) as u8).collect(),
    )
    .unwrap()
    .save(format!("out/{}.png", c));
  }

  let conv: GrayImage = (&kernels[92])
    .tap(|_| println!("Starting convolution"))
    .filter::<_, _, Luma<f32>>(&img, |out, res| *out = res / 5000.)
    .tap(|_| println!("Finished convolution"))
    .convert();
  conv.save("test.png");

  let convs: Vec<_> =
    kernels.iter().map(|ker| ker.filter::<_, _, Luma<f32>>(&img, |out, res| *out = res)).collect();
    
    let penalties: Vec<_> = kernels.iter().map(|ker| ker.data.iter().sum::<f32>() * penalty).collect();

  let mut out = String::new();
  for x in 0..w_img_char {
    for y in 0..h_img_char {
      out.push(
        chars
          .clone()
          .zip(convs.iter()).zip(penalties.iter())
          .max_by_key(|((c, conv), p)| {
            (conv
              .get_pixel(
                (w_char_px * (x as f32 + 0.5)) as u32,
                (h_char_px * (y as f32 + 0.5)) as u32,
              )
              .0[0] - **p)
              .pipe(FloatOrd)
          })
          .unwrap()
          .0.0,
      );
    }
    out.push('\n');
  }
  out
}

/// A wrapper around imageproc::filter::Kernel that owns it's data.
struct OwnedKernel<C, K> {
  data: C,
  width: u32,
  height: u32,
  _unused: std::marker::PhantomData<K>,
}

impl<'a, C: AsRef<[K]>, K: Num + Copy + 'a> OwnedKernel<C, K> {
  fn new(data: C, width: u32, height: u32) -> OwnedKernel<C, K> {
    Kernel::new(data.as_ref(), width, height); // Just to check assertions
    OwnedKernel::<C, K> { data, width, height, _unused: std::marker::PhantomData }
  }

  fn filter<P, F, Q>(&self, image: &Image<P>, mut f: F) -> Image<Q>
  where
    P: Pixel,
    <P as Pixel>::Subpixel: ValueInto<K>,
    Q: Pixel,
    F: FnMut(&mut Q::Subpixel, K),
  {
    Kernel::new(self.data.as_ref(), self.width, self.height).filter(image, f)
  }
}
