#[cfg(test)]

mod integration_tests {
    #[test]
    fn mock_register_fingerprint_to_capation() {
        println!("mock register fingerprint to capation start.");

        tools::logger::init_logger("crew");

        let common = cocrew::common::Common::new();
        let common = std::sync::Arc::new(std::sync::Mutex::new(common));
        let weak_common = std::sync::Arc::downgrade(&common);

        let receiver = cocrew::communicate::unpackager::FileReceiver::new(weak_common.clone());
        let receiver_ = receiver.clone();
        
        weak_common.upgrade().unwrap().lock().unwrap().file_receiver = Some(std::sync::Arc::new(std::sync::Mutex::new(receiver)));

        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        rt.spawn(async move {
            receiver_.init().await;
        });

        std::thread::sleep(std::time::Duration::from_secs(3));

        let common = crew::enter::Common::new();
        let common = std::sync::Arc::new(std::sync::Mutex::new(common));
        let weak_common = std::sync::Arc::downgrade(&common);

        let tasks = crew::roster::crews::TasksManager::new();
        let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(tasks));
        weak_common.upgrade().unwrap().lock().unwrap().tasks = Some(arc_tasks.clone());
        
        let roster = crew::roster::crews::ResourceList::new();
        let arc_roster = std::sync::Arc::new(std::sync::Mutex::new(roster));
        weak_common.upgrade().unwrap().lock().unwrap().roster = Some(arc_roster.clone());

        let mut handles = Vec::new();
        for i in 0..1 {
            let runtime = rt.handle().clone();
            let weak_common = weak_common.clone();
            let handle = rt.spawn(async move {
                println!("register_fingerprint_to_capation time {}", i);
                crew::fingerprint::register::register_fingerprint_to_capation(runtime.clone(), weak_common).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            });
            handles.push(handle);
        }

        rt.block_on(async {
            for handle in handles {
                handle.await.unwrap();
            }
        });
        
        let rosters = arc_roster.lock().unwrap().all();
        assert!(rosters.len() == 3);

        println!("mock register fingerprint to capation end.");
    }
}