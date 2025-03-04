#[cfg(test)]

mod integration_tests {

    #[test]
    fn register_same_fingerprint_to_captain_it() {
        println!("register same fingerprint to capation start.");

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
        for i in 0..3 {
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
        assert!(rosters.len() == 1);

        println!("register same fingerprint to capation end.");
    }

    #[test]
    fn register_multi_fingerprint_to_captain_it() {
        println!("register multi fingerprint to capation start.");
        tools::logger::init_logger("crew");

        let common = cocrew::common::Common::new();
        let common = std::sync::Arc::new(std::sync::Mutex::new(common));
        let weak_common = std::sync::Arc::downgrade(&common);

        let receiver = cocrew::communicate::unpackager::FileReceiver::
        new(weak_common.clone());
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
        for i in 0..2 {
            std::env::set_var("MOCK_USERNAME", std::format!("test_user_{}", i));
            std::env::set_var("MOCK_ALIASNAME", std::format!("test_alias_{}", i));
            std::env::set_var("MOCK_DEVICENAME", std::format!("test_device_{}", i));

            std::thread::sleep(std::time::Duration::from_secs(1));

            let runtime = rt.handle().clone();
            let weak_common = weak_common.clone();
            let handle = rt.spawn(async move {
                println!("register_fingerprint_to_capation time {}", i);
                crew::fingerprint::register::register_fingerprint_to_capation(runtime.clone(), weak_common).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            });
            handles.push(handle);
            let ten = std::time::Duration::from_secs(10);
            std::thread::sleep(ten);
        }

        rt.block_on(async {
            for handle in handles {
                handle.await.unwrap();
            }
        });


        let tasks = arc_tasks.lock().unwrap().all();
        assert!(tasks.len() == 2);

        let rosters = arc_roster.lock().unwrap().all();
        assert!(rosters.len() == 2);

        let dist = crew::communicate::distributor::Distributor::new(arc_tasks.clone(), arc_roster.clone());
        let arc_dist = std::sync::Arc::new(std::sync::Mutex::new(dist));
        arc_dist.lock().unwrap().schedule();
        arc_dist.lock().unwrap().schedule();
        arc_dist.lock().unwrap().schedule();

        let tasts = arc_tasks.lock().unwrap().all();
        for task in tasks {
            if task.username == "test_user_0"
            {
                assert!(task.running == 2);
            }
            if task.username == "test_user_1"
            {
                assert!(task.running == 1);
            }
            assert!(task.max == 100);
        }
        println!("tasts: {:?}", tasts);
        
        println!("register multi fingerprint to capation end.");
    }
}

