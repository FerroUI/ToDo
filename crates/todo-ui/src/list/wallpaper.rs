//! Original geometric wallpapers drawn by native FerroUI shapes. No external
//! photographs, network resources, or Microsoft artwork are bundled here.
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Geometry, IImage, Stretch};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct Scene {
    pub id: &'static str,
    pub name: &'static str,
    pub sky: &'static str,
    pub layers: &'static [(&'static str, &'static str)],
}

pub(crate) const SCENES: &[Scene] = &[
    Scene {
        id: "scene:mountains",
        name: "Mountains",
        sky: "#647B9D",
        layers: &[
            (
                "#91A3BC",
                "M0 520 L180 300 330 430 520 150 730 460 930 270 1200 500 1200 800 0 800Z",
            ),
            (
                "#435A7B",
                "M0 620 L220 450 410 570 700 300 940 580 1200 410 1200 800 0 800Z",
            ),
            (
                "#283F61",
                "M0 700 L260 560 480 720 780 510 1200 640 1200 800 0 800Z",
            ),
        ],
    },
    Scene {
        id: "scene:coast",
        name: "Coast",
        sky: "#438A9F",
        layers: &[
            (
                "#99C4CD",
                "M0 500 Q240 420 480 500 T960 500 T1440 500 L1200 800 0 800Z",
            ),
            (
                "#2D7089",
                "M0 570 Q200 490 500 590 T1100 580 L1200 800 0 800Z",
            ),
            (
                "#DBC4A1",
                "M0 760 Q380 600 650 710 T1200 620 L1200 800 0 800Z",
            ),
        ],
    },
    Scene {
        id: "scene:forest",
        name: "Forest",
        sky: "#517C70",
        layers: &[
            (
                "#85A99A",
                "M0 530 Q300 320 600 500 T1200 440 L1200 800 0 800Z",
            ),
            (
                "#315E52",
                "M0 680 L90 410 180 680 240 680 345 310 450 680 520 680 620 390 720 680 820 680 940 280 1060 680 1200 680 1200 800 0 800Z",
            ),
            (
                "#20493F",
                "M0 800 L170 490 330 800 490 800 600 520 730 800 880 800 1070 450 1200 750 1200 800Z",
            ),
        ],
    },
    Scene {
        id: "scene:dunes",
        name: "Dunes",
        sky: "#A37869",
        layers: &[
            (
                "#D9AC87",
                "M0 550 C220 310 420 650 700 490 S1050 360 1200 490 L1200 800 0 800Z",
            ),
            (
                "#B98266",
                "M0 650 C310 410 560 750 860 540 S1120 510 1200 540 L1200 800 0 800Z",
            ),
            (
                "#835B52",
                "M0 760 C260 560 470 730 660 720 S1050 600 1200 750 L1200 800 0 800Z",
            ),
        ],
    },
    Scene {
        id: "scene:lake",
        name: "Lake",
        sky: "#727EA1",
        layers: &[
            (
                "#A5A7BF",
                "M0 470 L250 300 480 470 780 250 1100 470 1200 470 1200 800 0 800Z",
            ),
            (
                "#475F84",
                "M0 540 L150 450 340 540 520 430 730 540 920 390 1200 540 1200 800 0 800Z",
            ),
            (
                "#8FADBA",
                "M0 610 Q320 570 600 620 T1200 600 L1200 800 0 800Z",
            ),
            (
                "#354E6D",
                "M0 800 L0 690 240 680 420 720 220 730Z M1200 800 L1200 650 920 690 1040 720 890 750Z",
            ),
        ],
    },
    Scene {
        id: "scene:night",
        name: "Night sky",
        sky: "#273657",
        layers: &[
            (
                "#CFD7D9",
                "M930 150 A65 65 0 1 0 1015 230 A68 68 0 0 1 930 150Z M170 220 L174 231 185 235 174 239 170 250 166 239 155 235 166 231Z M520 300 L523 308 531 311 523 314 520 322 517 314 509 311 517 308Z",
            ),
            (
                "#3D4A70",
                "M0 590 L180 450 380 570 610 350 810 600 990 440 1200 560 1200 800 0 800Z",
            ),
            (
                "#18263F",
                "M0 720 Q240 540 500 680 T1000 650 L1200 720 1200 800 0 800Z",
            ),
        ],
    },
];

ferroui_base::ferro_thread_local! {
    static GEOMETRIES: RefCell<HashMap<&'static str, Vec<ferroui_base::Ref<Geometry>>>> = RefCell::new(HashMap::new());
    static IMAGES: RefCell<Vec<(String, Rc<dyn IImage>)>> = RefCell::new(Vec::new());
}

pub(crate) fn view(value: &str, dark: bool) -> Option<Element> {
    if let Some(scene) = SCENES.iter().find(|scene| scene.id == value) {
        let shapes = GEOMETRIES.with(|cache| {
            cache
                .borrow_mut()
                .entry(scene.id)
                .or_insert_with(|| {
                    scene
                        .layers
                        .iter()
                        .map(|(_, path)| {
                            Geometry::parse(path).expect("original wallpaper geometry")
                        })
                        .collect()
                })
                .clone()
        });
        let mut canvas = canvas()
            .width(1200.0)
            .height(800.0)
            .background(scene.sky)
            .is_hit_test_visible(false);
        for ((color, _), geometry) in scene.layers.iter().zip(shapes) {
            canvas = canvas.child(
                path()
                    .data(Some(geometry))
                    .fill(*color)
                    .stretch(Stretch::None),
            );
        }
        return Some(
            grid()
                .is_hit_test_visible(false)
                .child(
                    viewbox()
                        .stretch(Stretch::UniformToFill)
                        .clip_to_bounds(true)
                        .set(ferroui_controls::Viewbox::child_property(), canvas),
                )
                .child(border().background(if dark { "#55000000" } else { "#10000000" }))
                .into(),
        );
    }
    let bitmap = IMAGES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((_, bitmap)) = cache.iter().find(|(key, _)| key == value) {
            return Some(bitmap.clone());
        }
        let bytes = todo_domain::backgrounds::decode_background(value).ok()?;
        let bitmap: Rc<dyn IImage> =
            Rc::new(Bitmap::from_stream(&mut std::io::Cursor::new(bytes)).ok()?);
        if cache.len() >= 3 {
            cache.remove(0);
        }
        cache.push((value.to_owned(), bitmap.clone()));
        Some(bitmap)
    });
    bitmap.map(|bitmap| {
        grid()
            .is_hit_test_visible(false)
            .clip_to_bounds(true)
            .child(image().source(Some(bitmap)).stretch(Stretch::UniformToFill))
            .child(border().background(if dark { "#88000000" } else { "#55000000" }))
            .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_catalog_uses_valid_native_paths_and_stable_portable_ids() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        assert_eq!(SCENES.len(), todo_domain::backgrounds::SCENES.len());
        for scene in SCENES {
            assert!(todo_domain::backgrounds::SCENES.contains(&scene.id));
            for (_, path) in scene.layers {
                assert!(
                    ferroui_base::media::PathGeometry::parse(path).is_ok(),
                    "{}: {path}",
                    scene.name
                );
            }
        }
        assert!(
            view("unknown-provider-background", false).is_none(),
            "unknown backgrounds must keep the normal readable header"
        );
        assert!(view("none", true).is_none());
    }
}
