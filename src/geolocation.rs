#![doc(test(no_crate_inject))]
#![doc(test(attr(no_run)))] 

//! Geolocation module for system location tracking
//!
//! This module provides cross-platform geolocation capabilities by:
//! 1. Detecting the system's public IP address
//! 2. Performing IP-based geolocation lookups
//! 3. Providing structured location information including:
//!    - Coordinates (latitude/longitude)
//!    - Country and region
//!    - City information
//!    - ISP/Organization details
//!
//! The module includes robust error handling and timeout support to ensure reliable operation
//! in various network conditions.

use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::timeout;

// Global storage for location history
static LOCATION_HISTORY: Lazy<Mutex<Vec<LocationInfo>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Struct containing comprehensive location information
///
/// This struct holds all available location data gathered through various geolocation methods,
/// including IP-based, WiFi-based, and system API methods.
///
/// # Fields
/// - `ip_address`: The system's public IP address
/// - `latitude`: Geographical latitude coordinate
/// - `longitude`: Geographical longitude coordinate
/// - `country`: Country name
/// - `region`: Administrative region/state
/// - `city`: City name
/// - `isp`: Internet Service Provider or organization name
/// - `wifi_ssid`: WiFi network SSID (if available)
/// - `wifi_bssid`: WiFi network BSSID/MAC address (if available)
/// - `system_location`: System-reported location (if available)
/// - `timestamp`: UTC timestamp when the location was last updated
/// - `source`: Source of the location data (IP, WiFi, System, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub ip_address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub wifi_ssid: Option<String>,
    pub wifi_bssid: Option<String>,
    pub system_location: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: LocationSource,
}

/// Custom error type for geolocation operations
///
/// This error type represents various failure scenarios that can occur during
/// geolocation operations, including network issues, IP detection failures,
/// and location lookup failures.
///
/// # Variants
/// - `NetworkError`: General network connectivity issues
/// - `IPDetectionFailed`: Failed to detect public IP address
/// - `LocationLookupFailed`: Failed to perform geolocation lookup
/// - `Timeout`: Operation timed out
/// - `FileError`: Error saving location data to file
/// - `WiFiError`: Error accessing WiFi information
/// - `SystemError`: Error accessing system location API
#[derive(Debug)]
pub enum GeolocationError {
    NetworkError(String),
    IPDetectionFailed(String),
    LocationLookupFailed(String),
    Timeout,
    FileError(String),
    WiFiError(String),
    SystemError(String),
    InvalidInput(String),
}

/// Source of the location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocationSource {
    IP,
    WiFi,
    System,
    Unknown,
}

/// Structure for WiFi-based location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiLocation {
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub signal_strength: Option<i32>,
    pub frequency: Option<f64>,
}

impl fmt::Display for GeolocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeolocationError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            GeolocationError::IPDetectionFailed(msg) => write!(f, "IP detection failed: {}", msg),
            GeolocationError::LocationLookupFailed(msg) => {
                write!(f, "Location lookup failed: {}", msg)
            }
            GeolocationError::Timeout => write!(f, "Operation timed out"),
            GeolocationError::FileError(msg) => write!(f, "File error: {}", msg),
            GeolocationError::WiFiError(msg) => write!(f, "WiFi error: {}", msg),
            GeolocationError::SystemError(msg) => write!(f, "System error: {}", msg),
            GeolocationError::InvalidInput(msg) => write!(f, "Invalid Input type error: {}", msg),
        }
    }
}

impl Error for GeolocationError {}

/// Trait defining the interface for geolocation services
#[async_trait::async_trait]
pub trait GeolocatorTrait {
    /// Gets the public IP address of the system
    async fn get_public_ip(&self) -> Result<String, GeolocationError>;

    /// Gets location information based on IP address
    async fn get_location_info(&self) -> Result<LocationInfo, GeolocationError>;
}

/// Geolocation tracker for obtaining system location information
///
/// This struct provides methods for detecting the system's public IP address
/// and performing geolocation lookups. It includes timeout support to prevent
/// hanging operations and robust error handling.
///
/// # Features
/// - Configurable operation timeout
/// - Public IP address detection
/// - IP-based geolocation lookups
/// - Human-readable location formatting
pub struct Geolocator {
    timeout_seconds: u64,
}

impl Default for Geolocator {
    fn default() -> Self {
        Self::new(10) // Default 10-second timeout
    }
}

impl Geolocator {
    /// Creates a new Geolocator instance with the specified timeout duration
    ///
    /// The timeout parameter controls how long the geolocation operations will
    /// wait before failing. This helps prevent hanging operations in case of
    /// network issues or slow responses.
    ///
    /// # Arguments
    /// * `timeout_seconds` - Number of seconds before timing out operations
    ///
    /// # Examples
    /// ```no_run
    /// // Example: let geolocator = Geolocator::new(5); // 5-second timeout
    /// ```
    pub fn new(timeout_seconds: u64) -> Self {
        Geolocator { timeout_seconds }
    }

    /// Gets location information using multiple methods
    ///
    /// This method attempts to gather location information using multiple methods:
    /// 1. IP-based geolocation
    /// 2. WiFi-based geolocation (if available)
    /// 3. System location API (if available)
    ///
    /// It returns the most accurate location information available from any source.
    ///
    /// # Returns
    /// * `Ok(LocationInfo)` - Structured location information on success
    /// * `Err(GeolocationError)` - If all methods fail
    ///
    /// # Examples
    /// ```no_run
    /// // Example: Using the multi-source location functionality
    /// // let location = geolocator.get_multi_source_location().await?;
    /// ```
    pub async fn get_multi_source_location(&self) -> Result<LocationInfo, GeolocationError> {
        // Try IP-based geolocation
        let mut location = match self.get_ip_location().await {
            Ok(loc) => loc,
            Err(_) => LocationInfo {
                ip_address: None,
                latitude: None,
                longitude: None,
                country: None,
                region: None,
                city: None,
                isp: None,
                wifi_ssid: None,
                wifi_bssid: None,
                system_location: None,
                timestamp: chrono::Utc::now(),
                source: LocationSource::Unknown,
            },
        };

        // Try WiFi-based geolocation
        if let Ok(wifi_info) = self.get_wifi_location() {
            location.wifi_ssid = wifi_info.ssid;
            location.wifi_bssid = wifi_info.bssid;
        }

        // Try system location API
        if let Ok(system_loc) = self.get_system_location() {
            location.system_location = Some(system_loc);
        }

        // Save to history
        self.save_location_to_history(&location)?;

        Ok(location)
    }

    /// Gets WiFi-based location information
    ///
    /// This method attempts to get location information from the connected WiFi network.
    /// Note: Requires appropriate permissions on the system.
    ///
    /// # Returns
    /// * `Ok(WifiLocation)` - WiFi network information
    /// * `Err(GeolocationError)` - If WiFi information cannot be accessed
    fn get_wifi_location(&self) -> Result<WifiLocation, GeolocationError> {
        // TODO: Implement WiFi location detection
        // This will require platform-specific code
        Err(GeolocationError::WiFiError(
            "WiFi location not implemented yet".to_string(),
        ))
    }

    /// Gets system-reported location information
    ///
    /// This method attempts to get location information from the system's location API.
    /// Note: Requires appropriate permissions on the system.
    ///
    /// # Returns
    /// * `Ok(String)` - System-reported location string
    /// * `Err(GeolocationError)` - If system location cannot be accessed
    fn get_system_location(&self) -> Result<String, GeolocationError> {
        // TODO: Implement system location API access
        // This will require platform-specific code
        Err(GeolocationError::SystemError(
            "System location not implemented yet".to_string(),
        ))
    }

    /// Saves location information to history and file
    ///
    /// This method saves the location information to both in-memory history and a persistent file.
    /// The file is stored in the user's home directory under .frida/locations.json
    ///
    /// # Arguments
    /// * `location` - Location information to save
    ///
    /// # Returns
    /// * `Ok(())` - On successful save
    /// * `Err(GeolocationError)` - If save fails
    fn save_location_to_history(&self, location: &LocationInfo) -> Result<(), GeolocationError> {
        // Save to in-memory history
        LOCATION_HISTORY.lock().unwrap().push(location.clone());

        // Save to file
        let locations_dir = dirs::home_dir()
            .ok_or_else(|| GeolocationError::FileError("Cannot get home directory".to_string()))?
            .join(".frida");

        fs::create_dir_all(&locations_dir)
            .map_err(|e| GeolocationError::FileError(format!("Cannot create directory: {}", e)))?;

        let locations_file = locations_dir.join("locations.json");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&locations_file)
            .map_err(|e| GeolocationError::FileError(format!("Cannot open file: {}", e)))?;

        let location_json = serde_json::to_string(&location)
            .map_err(|e| GeolocationError::FileError(format!("Cannot serialize: {}", e)))?;

        writeln!(file, "{}", location_json)
            .map_err(|e| GeolocationError::FileError(format!("Cannot write to file: {}", e)))?;

        Ok(())
    }

    /// Loads location history from file
    ///
    /// This method loads previously saved location information from the persistent file.
    ///
    /// # Returns
    /// * `Ok(Vec<LocationInfo>)` - Vector of all saved locations
    /// * `Err(GeolocationError)` - If file cannot be read
    pub fn load_location_history() -> Result<Vec<LocationInfo>, GeolocationError> {
        let locations_dir = dirs::home_dir()
            .ok_or_else(|| GeolocationError::FileError("Cannot get home directory".to_string()))?
            .join(".frida");

        let locations_file = locations_dir.join("locations.json");

        if !locations_file.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&locations_file)
            .map_err(|e| GeolocationError::FileError(format!("Cannot read file: {}", e)))?;

        let locations: Vec<LocationInfo> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(locations)
    }

    /// Clears location history from both memory and file
    ///
    /// This method removes all stored location information.
    ///
    /// # Returns
    /// * `Ok(())` - On successful clear
    /// * `Err(GeolocationError)` - If file cannot be cleared
    pub fn clear_location_history() -> Result<(), GeolocationError> {
        *LOCATION_HISTORY.lock().unwrap() = Vec::new();

        let locations_dir = dirs::home_dir()
            .ok_or_else(|| GeolocationError::FileError("Cannot get home directory".to_string()))?
            .join(".frida");

        let locations_file = locations_dir.join("locations.json");

        if locations_file.exists() {
            fs::remove_file(&locations_file)
                .map_err(|e| GeolocationError::FileError(format!("Cannot remove file: {}", e)))?;
        }

        Ok(())
    }

    /// Asynchronously retrieves the system's public IP address
    ///
    /// This method performs a network request to determine the system's public
    /// IP address as seen from the internet.
    ///
    /// # Returns
    /// * `Ok(String)` - The public IP address on success
    /// * `Err(GeolocationError)` - If IP detection fails or times out
    ///
    /// # Examples
    /// ```no_run
    /// // Example: Getting the public IP address
    /// // let ip = geolocator.get_public_ip().await?;
    /// ```
    pub async fn get_public_ip(&self) -> Result<String, GeolocationError> {
        match timeout(Duration::from_secs(self.timeout_seconds), public_ip::addr()).await {
            Ok(result) => match result {
                Some(ip) => Ok(ip.to_string()),
                None => Err(GeolocationError::IPDetectionFailed(
                    "No public IP address found".to_string(),
                )),
            },
            Err(_) => Err(GeolocationError::Timeout),
        }
    }

    /// Performs a geolocation lookup based on the system's public IP address
    ///
    /// This method combines IP detection with geolocation services to provide
    /// comprehensive location information including coordinates, country,
    /// region, city, and ISP details.
    ///
    /// # Returns
    /// * `Ok(LocationInfo)` - Structured location information on success
    /// * `Err(GeolocationError)` - If any part of the lookup fails
    ///
    /// # Examples
    /// ```no_run
    /// // Example: Getting IP-based location information
    /// // let location = geolocator.get_ip_location().await?;
    /// ```
    /// Gets IP-based location information
    ///
    /// This method is a convenience wrapper around get_location_info that explicitly marks
    /// the source of the location data as IP-based.
    ///
    /// # Returns
    /// * `Ok(LocationInfo)` - Structured location information on success
    /// * `Err(GeolocationError)` - If IP-based location lookup fails
    pub async fn get_ip_location(&self) -> Result<LocationInfo, GeolocationError> {
        self.get_location_info().await
    }

    async fn get_location_info(&self) -> Result<LocationInfo, GeolocationError> {
        // Get public IP first
        let ip = self.get_public_ip().await?;

        // Use iplocate to get geolocation data from the IP
        let ip_addr = ip
            .parse()
            .map_err(|_| GeolocationError::InvalidInput("Invalid IP address".to_string()))?;

        match timeout(Duration::from_secs(self.timeout_seconds), async {
            iplocate::lookup(ip_addr)
        })
        .await
        {
            Ok(location_result) => match location_result {
                Ok(location) => Ok(LocationInfo {
                    ip_address: Some(ip),
                    latitude: location.geo_ip.latitude,
                    longitude: location.geo_ip.longitude,
                    country: location.geo_ip.country,
                    region: location.geo_ip.continent,
                    city: location.geo_ip.city,
                    isp: location.geo_ip.org,
                    wifi_ssid: None,
                    wifi_bssid: None,
                    system_location: None,
                    timestamp: chrono::Utc::now(),
                    source: LocationSource::IP,
                }),
                Err(e) => Err(GeolocationError::LocationLookupFailed(e.to_string())),
            },
            Err(_) => Err(GeolocationError::Timeout),
        }
    }

    /// Converts location information into a human-readable string format
    ///
    /// This utility function takes a LocationInfo struct and formats it into
    /// a nicely formatted string that includes all available location details.
    ///
    /// # Arguments
    /// * `location` - Reference to a LocationInfo struct to format
    ///
    /// # Returns
    /// A formatted string containing all available location information
    pub fn format_location(location: &LocationInfo) -> String {
        let mut result = String::new();

        if let Some(ip) = &location.ip_address {
            result.push_str(&format!("IP Address: {}\n", ip));
        }

        if let (Some(lat), Some(lon)) = (location.latitude, location.longitude) {
            result.push_str(&format!("Coordinates: {:.6}, {:.6}\n", lat, lon));
        }

        if let Some(city) = &location.city {
            result.push_str(&format!("City: {}\n", city));
        }

        if let Some(region) = &location.region {
            result.push_str(&format!("Region: {}\n", region));
        }

        if let Some(country) = &location.country {
            result.push_str(&format!("Country: {}\n", country));
        }

        if let Some(isp) = &location.isp {
            result.push_str(&format!("ISP/Organization: {}\n", isp));
        }

        result.push_str(&format!("Timestamp: {}\n", location.timestamp));

        result
    }
}

#[async_trait]
impl GeolocatorTrait for Geolocator {
    async fn get_public_ip(&self) -> Result<String, GeolocationError> {
        match timeout(Duration::from_secs(self.timeout_seconds), public_ip::addr()).await {
            Ok(result) => match result {
                Some(ip) => Ok(ip.to_string()),
                None => Err(GeolocationError::IPDetectionFailed(
                    "No public IP address found".to_string(),
                )),
            },
            Err(_) => Err(GeolocationError::Timeout),
        }
    }

    async fn get_location_info(&self) -> Result<LocationInfo, GeolocationError> {
        // Get public IP first
        let ip = self.get_public_ip().await?;

        // Use iplocate to get geolocation data from the IP
        let ip_addr = ip
            .parse()
            .map_err(|_| GeolocationError::InvalidInput("Invalid IP address".to_string()))?;

        match timeout(Duration::from_secs(self.timeout_seconds), async {
            iplocate::lookup(ip_addr)
        })
        .await
        {
            Ok(location_result) => match location_result {
                Ok(location) => Ok(LocationInfo {
                    ip_address: Some(ip),
                    latitude: location.geo_ip.latitude,
                    longitude: location.geo_ip.longitude,
                    country: location.geo_ip.country,
                    region: location.geo_ip.continent,
                    city: location.geo_ip.city,
                    isp: location.geo_ip.org,
                    wifi_ssid: None,
                    wifi_bssid: None,
                    system_location: None,
                    timestamp: chrono::Utc::now(),
                    source: LocationSource::IP,
                }),
                Err(e) => Err(GeolocationError::LocationLookupFailed(e.to_string())),
            },
            Err(_) => Err(GeolocationError::Timeout),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::*;

    mock! {
        pub GeolocatorMock {}

        #[async_trait::async_trait]
        impl GeolocatorTrait for GeolocatorMock {
            async fn get_public_ip(&self) -> Result<String, GeolocationError>;
            async fn get_location_info(&self) -> Result<LocationInfo, GeolocationError>;
        }
    }

    #[tokio::test]
    async fn test_format_location() {
        let location = LocationInfo {
            ip_address: Some("192.168.1.1".to_string()),
            latitude: Some(40.7128),
            longitude: Some(-74.0060),
            country: Some("United States".to_string()),
            region: Some("New York".to_string()),
            city: Some("New York City".to_string()),
            isp: Some("Example ISP".to_string()),
            wifi_ssid: None,
            wifi_bssid: None,
            system_location: None,
            timestamp: chrono::Utc::now(),
            source: LocationSource::IP,
        };

        let formatted = Geolocator::format_location(&location);
        assert!(formatted.contains("IP Address: 192.168.1.1"));
        assert!(formatted.contains("Coordinates: 40.712800, -74.006000"));
        assert!(formatted.contains("City: New York City"));
        assert!(formatted.contains("Region: New York"));
        assert!(formatted.contains("Country: United States"));
        assert!(formatted.contains("ISP/Organization: Example ISP"));
        assert!(formatted.contains("Timestamp:"));
    }
}
