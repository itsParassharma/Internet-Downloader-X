//! Native resource templates derived from the supplied executable.
//! Geometry, fonts, class names and IDs are retained; branding/menu references are adapted.
//! Original resources and provenance remain under reverse-engineering/native-ui-resources.
#![allow(dead_code)]
pub(super) fn dialog(id: u32) -> Vec<u32> {
    let bytes: &[u8] = match id {
        100 => include_bytes!("../../assets/native/dialog_100.bin"),
        102 => include_bytes!("../../assets/native/dialog_102.bin"),
        103 => include_bytes!("../../assets/native/dialog_103.bin"),
        105 => include_bytes!("../../assets/native/dialog_105.bin"),
        130 => include_bytes!("../../assets/native/dialog_130.bin"),
        135 => include_bytes!("../../assets/native/dialog_135.bin"),
        139 => include_bytes!("../../assets/native/dialog_139.bin"),
        140 => include_bytes!("../../assets/native/dialog_140.bin"),
        141 => include_bytes!("../../assets/native/dialog_141.bin"),
        143 => include_bytes!("../../assets/native/dialog_143.bin"),
        145 => include_bytes!("../../assets/native/dialog_145.bin"),
        146 => include_bytes!("../../assets/native/dialog_146.bin"),
        147 => include_bytes!("../../assets/native/dialog_147.bin"),
        148 => include_bytes!("../../assets/native/dialog_148.bin"),
        150 => include_bytes!("../../assets/native/dialog_150.bin"),
        184 => include_bytes!("../../assets/native/dialog_184.bin"),
        185 => include_bytes!("../../assets/native/dialog_185.bin"),
        186 => include_bytes!("../../assets/native/dialog_186.bin"),
        187 => include_bytes!("../../assets/native/dialog_187.bin"),
        191 => include_bytes!("../../assets/native/dialog_191.bin"),
        193 => include_bytes!("../../assets/native/dialog_193.bin"),
        194 => include_bytes!("../../assets/native/dialog_194.bin"),
        195 => include_bytes!("../../assets/native/dialog_195.bin"),
        196 => include_bytes!("../../assets/native/dialog_196.bin"),
        204 => include_bytes!("../../assets/native/dialog_204.bin"),
        208 => include_bytes!("../../assets/native/dialog_208.bin"),
        209 => include_bytes!("../../assets/native/dialog_209.bin"),
        226 => include_bytes!("../../assets/native/dialog_226.bin"),
        228 => include_bytes!("../../assets/native/dialog_228.bin"),
        229 => include_bytes!("../../assets/native/dialog_229.bin"),
        230 => include_bytes!("../../assets/native/dialog_230.bin"),
        231 => include_bytes!("../../assets/native/dialog_231.bin"),
        232 => include_bytes!("../../assets/native/dialog_232.bin"),
        236 => include_bytes!("../../assets/native/dialog_236.bin"),
        239 => include_bytes!("../../assets/native/dialog_239.bin"),
        240 => include_bytes!("../../assets/native/dialog_240.bin"),
        241 => include_bytes!("../../assets/native/dialog_241.bin"),
        242 => include_bytes!("../../assets/native/dialog_242.bin"),
        244 => include_bytes!("../../assets/native/dialog_244.bin"),
        245 => include_bytes!("../../assets/native/dialog_245.bin"),
        257 => include_bytes!("../../assets/native/dialog_257.bin"),
        259 => include_bytes!("../../assets/native/dialog_259.bin"),
        262 => include_bytes!("../../assets/native/dialog_262.bin"),
        264 => include_bytes!("../../assets/native/dialog_264.bin"),
        268 => include_bytes!("../../assets/native/dialog_268.bin"),
        269 => include_bytes!("../../assets/native/dialog_269.bin"),
        270 => include_bytes!("../../assets/native/dialog_270.bin"),
        271 => include_bytes!("../../assets/native/dialog_271.bin"),
        279 => include_bytes!("../../assets/native/dialog_279.bin"),
        280 => include_bytes!("../../assets/native/dialog_280.bin"),
        281 => include_bytes!("../../assets/native/dialog_281.bin"),
        284 => include_bytes!("../../assets/native/dialog_284.bin"),
        286 => include_bytes!("../../assets/native/dialog_286.bin"),
        291 => include_bytes!("../../assets/native/dialog_291.bin"),
        292 => include_bytes!("../../assets/native/dialog_292.bin"),
        294 => include_bytes!("../../assets/native/dialog_294.bin"),
        296 => include_bytes!("../../assets/native/dialog_296.bin"),
        297 => include_bytes!("../../assets/native/dialog_297.bin"),
        298 => include_bytes!("../../assets/native/dialog_298.bin"),
        300 => include_bytes!("../../assets/native/dialog_300.bin"),
        302 => include_bytes!("../../assets/native/dialog_302.bin"),
        303 => include_bytes!("../../assets/native/dialog_303.bin"),
        304 => include_bytes!("../../assets/native/dialog_304.bin"),
        305 => include_bytes!("../../assets/native/dialog_305.bin"),
        306 => include_bytes!("../../assets/native/dialog_306.bin"),
        307 => include_bytes!("../../assets/native/dialog_307.bin"),
        308 => include_bytes!("../../assets/native/dialog_308.bin"),
        309 => include_bytes!("../../assets/native/dialog_309.bin"),
        310 => include_bytes!("../../assets/native/dialog_310.bin"),
        332 => include_bytes!("../../assets/native/dialog_332.bin"),
        334 => include_bytes!("../../assets/native/dialog_334.bin"),
        335 => include_bytes!("../../assets/native/dialog_335.bin"),
        336 => include_bytes!("../../assets/native/dialog_336.bin"),
        337 => include_bytes!("../../assets/native/dialog_337.bin"),
        338 => include_bytes!("../../assets/native/dialog_338.bin"),
        339 => include_bytes!("../../assets/native/dialog_339.bin"),
        340 => include_bytes!("../../assets/native/dialog_340.bin"),
        341 => include_bytes!("../../assets/native/dialog_341.bin"),
        342 => include_bytes!("../../assets/native/dialog_342.bin"),
        344 => include_bytes!("../../assets/native/dialog_344.bin"),
        345 => include_bytes!("../../assets/native/dialog_345.bin"),
        348 => include_bytes!("../../assets/native/dialog_348.bin"),
        349 => include_bytes!("../../assets/native/dialog_349.bin"),
        352 => include_bytes!("../../assets/native/dialog_352.bin"),
        363 => include_bytes!("../../assets/native/dialog_363.bin"),
        364 => include_bytes!("../../assets/native/dialog_364.bin"),
        365 => include_bytes!("../../assets/native/dialog_365.bin"),
        367 => include_bytes!("../../assets/native/dialog_367.bin"),
        369 => include_bytes!("../../assets/native/dialog_369.bin"),
        370 => include_bytes!("../../assets/native/dialog_370.bin"),
        381 => include_bytes!("../../assets/native/dialog_381.bin"),
        487 => include_bytes!("../../assets/native/dialog_487.bin"),
        488 => include_bytes!("../../assets/native/dialog_488.bin"),
        489 => include_bytes!("../../assets/native/dialog_489.bin"),
        494 => include_bytes!("../../assets/native/dialog_494.bin"),
        504 => include_bytes!("../../assets/native/dialog_504.bin"),
        505 => include_bytes!("../../assets/native/dialog_505.bin"),
        506 => include_bytes!("../../assets/native/dialog_506.bin"),
        30721 => include_bytes!("../../assets/native/dialog_30721.bin"),
        30734 => include_bytes!("../../assets/native/dialog_30734.bin"),
        _ => return Vec::new(),
    };
    bytes
        .chunks(4)
        .map(|chunk| {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            u32::from_le_bytes(word)
        })
        .collect()
}
pub(super) fn top_level(id: u32) -> Vec<u32> {
    let mut words = dialog(id);
    if words.is_empty() {
        return words;
    }
    let extended = words[0] == 0xffff0001;
    let index = if extended { 3 } else { 0 };
    words[index] = (words[index] & !(0x40000000 | 0x00000400)) | 0x80c80000;
    words
}
pub(super) fn menu(id: u32) -> Vec<u16> {
    let bytes: &[u8] = match id {
        129 => include_bytes!("../../assets/native/menu_129.bin"),
        256 => include_bytes!("../../assets/native/menu_256.bin"),
        _ => return Vec::new(),
    };
    bytes
        .chunks_exact(2)
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect()
}
