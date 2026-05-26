//
//  LocationManager.swift
//  clawparty_iOS
//
//  Created by 林东臣 on 2024/9/16.
//

import Foundation
import UIKit
import CoreLocation

class LocationManager: NSObject, CLLocationManagerDelegate {
    private let locationManager = CLLocationManager()

    override init() {
        super.init()

        locationManager.delegate = self
        locationManager.requestWhenInUseAuthorization()
        locationManager.requestAlwaysAuthorization()
        locationManager.allowsBackgroundLocationUpdates = true
        locationManager.pausesLocationUpdatesAutomatically = false
    }

    func startLocationUpdates() {
        locationManager.startUpdatingLocation()
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        if let location = locations.last {
            NSLog("定位更新: \(location.coordinate.latitude), \(location.coordinate.longitude)")
        }
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        NSLog("定位失败: \(error.localizedDescription)")
    }
}
