//! Spec 100 §8: the `ImageDecoder` port draws a model for an export.

use gafira_doc::{ContentHash, Model3dFormat, Model3dNode, timeline::AssetRef};
use gafira_io_model3d::ModelDecoder;
use gafira_ports::export::ImageDecoder;
use gafira_ports::media::MediaError;
use gafira_raster::{ImageBuf, PixelBuf};

struct NoImages;

impl ImageDecoder for NoImages {
    fn decode(&self, _bytes: &[u8]) -> Result<ImageBuf, MediaError> {
        Err(MediaError::Corrupt {
            detail: "none".into(),
        })
    }
}

const CUBE: &str = "v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\nf 1 2 3 4\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n";

// Spec 100 AC-12: the picture has the model in the middle and nothing around it.
#[test]
fn s100_the_decoder_draws_a_model_on_transparency() {
    let decoder = ModelDecoder::new(NoImages);
    let model = Model3dNode::new(AssetRef::new(ContentHash([1; 32])), Model3dFormat::Obj);
    let img = decoder
        .render_model(&model, CUBE.as_bytes(), (80, 60))
        .expect("drawn");
    assert_eq!((img.width(), img.height()), (80, 60));
    let PixelBuf::Rgba8(px) = img.data() else {
        panic!("rgba8")
    };
    let at = |x: usize, y: usize| px[(y * 80 + x) * 4 + 3];
    assert_eq!(at(0, 0), 0);
    assert_eq!(at(79, 59), 0);
    assert_eq!(at(40, 30), 255);
    // The second picture of the same file reuses the read model and gives the same pixels.
    let again = decoder
        .render_model(&model, CUBE.as_bytes(), (80, 60))
        .expect("drawn");
    assert_eq!(again, img);
    // A broken file is an error, not a panic.
    let broken = Model3dNode::new(AssetRef::new(ContentHash([2; 32])), Model3dFormat::Glb);
    assert!(
        decoder
            .render_model(&broken, b"glTF\x02\0\0\0", (10, 10))
            .is_err()
    );
}
