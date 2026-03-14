import Foundation
import UIKit
import ActivityKit
import UserNotifications
import WidgetKit
import SwiftUI
import BackgroundTasks
import NetworkExtension

///=====

// 声明 C 函数
@_silgen_name("pipy_main") func pipy_main(argc: Int32, argv: UnsafeMutablePointer<UnsafeMutablePointer<Int8>?>) -> Int32

@objc class ActivityHelper: NSObject {
    
    static var timer: Timer? // 保存计时器
    static var locationManager: LocationManager?
    
    @objc static func startLiveActivity() {
        NSLog("调试-startLiveActivity called")
    }
    
    @objc static func playPipy() {
        // Get the document directory path
        var bgTask: UIBackgroundTaskIdentifier = .invalid
        bgTask = UIApplication.shared.beginBackgroundTask {
            // 当后台任务即将过期时调用
            UIApplication.shared.endBackgroundTask(bgTask)
            bgTask = .invalid
        }
        DispatchQueue.global(qos: .default).async {
            for progress in stride(from: 0.0, to: 100.0, by: 1.0) {
                let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
                guard let documentDirectory = paths.first else {
                    NSLog("调试-Could not find document directory")
                    return
                }
                
                let ztmdbPath = documentDirectory.appendingPathComponent("ztmdb").path
                let ztmlogPath = documentDirectory.appendingPathComponent("log.txt").path
                NSLog("调试-libpipy documentDirectory is \(documentDirectory.path)")
                
                // Create the argument list
                let args = ["--pipy", "repo://ztm/agent", "--args", "--data", ztmdbPath, "--listen", "6789", "--pipy-options", "--log-file=\(ztmlogPath)"]
                
                let cArgs = args.map { strdup($0) }
                defer { cArgs.forEach { free($0) } }  // 确保释放内存
                
                // 创建 UnsafeMutablePointer 数组
                let argv = UnsafeMutablePointer<UnsafeMutablePointer<Int8>?>.allocate(capacity: cArgs.count)
                for (index, arg) in cArgs.enumerated() {
                    argv[index] = arg
                }
                argv[cArgs.count] = nil // 添加 null 终止符
                
                // Call the pipy_main function
                let result = pipy_main(argc: Int32(cArgs.count), argv: argv)
                NSLog("调试-callPipyMain pipy_main returned: \(result)")
            }
            UIApplication.shared.endBackgroundTask(bgTask)
        }
    }

    @objc static func scheduleBackgroundTask() {
        let request = BGProcessingTaskRequest(identifier: "com.flomesh.clawparty.pipy")
        request.requiresNetworkConnectivity = true  // 需要网络连接
        request.requiresExternalPower = false       // 不需要外部电源

        do {
            try BGTaskScheduler.shared.submit(request)
            NSLog("调试-后台处理任务提交成功")
        } catch {
            NSLog("提交后台处理任务失败: \(error)")
        }
    }
    @objc static func handleBackgroundTask(_ task: BGProcessingTask) {
        // 确保后台任务不会被立即挂起
        task.expirationHandler = {
            // 任务到期时调用
            task.setTaskCompleted(success: false)
            scheduleBackgroundTask()  // 重新调度任务
        }
        
        // 完成任务后调用
        task.setTaskCompleted(success: true)
    }

    // 启动计时器，每10秒更新一次锁屏内容
    @objc static func startTimer() {
        DispatchQueue.main.async {
            NSLog("调试-注册更新实时活动 DispatchQueue")
            self.timer = Timer.scheduledTimer(withTimeInterval: 60.0, repeats: true) { _ in
                locationManager?.startLocationUpdates();
//                self.playPipy();   // 启动 Pipy
                self.scheduleBackgroundTask();
//                self.startBackgroundDownload();
            }
        }
    }
    
    
    @objc static func watchEvent() {
        
        // 监听应用程序从后台切换到前台的事件
        NotificationCenter.default.addObserver(self, selector: #selector(handleAppOpen), name: UIApplication.willEnterForegroundNotification, object: nil)
        
        // 监听应用冷启动
        NotificationCenter.default.addObserver(self, selector: #selector(handleAppLaunch), name: UIApplication.didFinishLaunchingNotification, object: nil)
        
        NotificationCenter.default.addObserver(self, selector: #selector(handleDidEnterBackground), name: UIApplication.didEnterBackgroundNotification, object: nil)
                
        
    }

    @objc static func handleDidEnterBackground() {
        locationManager?.startLocationUpdates()
//        self.startBackgroundDownload();
        
    }
    @objc static func handleAppOpen() {
        locationManager?.startLocationUpdates();
//        self.startBackgroundDownload();
        // self.playPipy();
        
    }

    @objc static func handleAppLaunch() {
        locationManager = LocationManager()
        locationManager?.startLocationUpdates();
        self.playPipy();
    }
    
}
@objc class WidgetHelper: NSObject {
    @objc static func reloadWidgets() {
        if #available(iOS 16.1, *) {
            NSLog("调试-Widget timelines before")
//            let widget = AdventureActivityConfiguration();
            WidgetCenter.shared.reloadAllTimelines()
            WidgetCenter.shared.reloadTimelines(ofKind: "RunnerWidgetAttributes")
            NSLog("调试-Widget timelines reloaded")
        } else {
            NSLog("调试-iOS version is too low to reload widgets")
        }
    }
}

