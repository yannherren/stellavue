fn main() {
    embuild::espidf::sysenv::output();
    println!("cargo:rustc-env=WIFI_SSID=Stellavue"); // Not important env variables
    println!("cargo:rustc-env=WIFI_PASSWORD=iseestars"); // Only for the local wifi of the tracker
}
