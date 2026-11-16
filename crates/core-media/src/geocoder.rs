use reverse_geocoder::ReverseGeocoder;
use std::sync::OnceLock;

struct CoastalLandmark {
    name: &'static str,
    lat: f64,
    lon: f64,
    radius_km: f64,
}

const LANDMARKS: &[CoastalLandmark] = &[
    CoastalLandmark {
        name: "Kuta Beach Bali",
        lat: -8.7185,
        lon: 115.1686,
        radius_km: 20.0,
    },
    CoastalLandmark {
        name: "Ubud Bali",
        lat: -8.5069,
        lon: 115.2625,
        radius_km: 15.0,
    },
    CoastalLandmark {
        name: "Ancol Beach Jakarta",
        lat: -6.1256,
        lon: 106.842,
        radius_km: 12.0,
    },
    CoastalLandmark {
        name: "Santa Monica Beach",
        lat: 34.0195,
        lon: -118.4912,
        radius_km: 15.0,
    },
    CoastalLandmark {
        name: "Bondi Beach Sydney",
        lat: -33.8915,
        lon: 151.2767,
        radius_km: 15.0,
    },
];

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0_f64;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    r * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())
}

fn geocoder() -> &'static ReverseGeocoder {
    static GEOCODER: OnceLock<ReverseGeocoder> = OnceLock::new();
    GEOCODER.get_or_init(ReverseGeocoder::new)
}

/// Resolves GPS coordinates offline using coastal landmarks + GeoNames city lookup.
pub fn lookup_place_offline(lat: Option<f64>, lon: Option<f64>) -> Option<String> {
    let (lat, lon) = (lat?, lon?);
    if !lat.is_finite() || !lon.is_finite() {
        return None;
    }

    let mut best_landmark: Option<(&'static str, f64)> = None;
    for lm in LANDMARKS {
        let dist = haversine_km(lat, lon, lm.lat, lm.lon);
        if dist <= lm.radius_km && best_landmark.is_none_or(|(_, d)| dist < d) {
            best_landmark = Some((lm.name, dist));
        }
    }
    if let Some((name, _)) = best_landmark {
        return Some(name.to_string());
    }

    let res = geocoder().search((lat, lon));
    Some(res.record.name.clone())
}
