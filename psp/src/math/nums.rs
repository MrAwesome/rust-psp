#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn fmaxf(x: f32, y: f32) -> f32 {
    if x < y {
        y
    } else {
        x
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn fminf(x: f32, y: f32) -> f32 {
    if x > y {
        y
    } else {
        x
    }
}
