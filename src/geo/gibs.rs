use image::RgbImage;

pub const LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR: &str =
    "MODIS_Terra_CorrectedReflectance_TrueColor";

pub struct GibsEpsg4326Params {
    pub layers: String,
    pub bbox: (f32, f32, f32, f32),
    pub size: (usize, usize),
}

fn gibs_epsg4326_url(params: &GibsEpsg4326Params) -> String {
    let (min_lon, min_lat, max_lon, max_lat) = params.bbox;
    let (width, height) = params.size;
    let layers = &params.layers;

    format!(
        "https://gibs.earthdata.nasa.gov/wms/epsg4326/best/wms.cgi?\
    version=1.3.0&service=WMS&request=GetMap&format=image/png\
    &STYLE=default&bbox={min_lat:.6},{min_lon:.6},{max_lat:.6},{max_lon:.6}&CRS=EPSG:4326\
    &HEIGHT={height}&WIDTH={width}\
    &TIME=2021-03-01&layers={layers}"
    )
}

pub async fn fetch_epsg4326_image(
    client: &reqwest::Client,
    params: GibsEpsg4326Params,
) -> Result<RgbImage, reqwest::Error> {
    let response = client
        .get(gibs_epsg4326_url(&params))
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?.to_vec();
    let img = image::load_from_memory(&bytes).unwrap();
    Ok(img.to_rgb8())
}
