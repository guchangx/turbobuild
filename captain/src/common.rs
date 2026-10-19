
#[derive(Default)]
pub struct Common {
    pub notification: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::communicate::receiver::NotificationReceiver>>>,
    pub constitutions: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ConstitutionList>>>,
    pub resources: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::ResourceList>>>,
    pub tasks: std::option::Option<std::sync::Arc<std::sync::Mutex<crate::roster::crews::TaskManager>>>,
}

impl Common {
    pub fn new() -> Self {
        
        let common = Common {
            notification: None,
            constitutions: None,
            resources: None,
            tasks: None,
        };
        
        return common;
    }
}

pub fn init_common() {
    let common = Common::new();
    let common = std::sync::Arc::new(std::sync::Mutex::new(common));
    let weak_common = std::sync::Arc::downgrade(&common);

    let constitutions = crate::roster::crews::ConstitutionList::new(weak_common.clone());
    let arc_constitutions = std::sync::Arc::new(std::sync::Mutex::new(constitutions));
    weak_common.upgrade().unwrap().lock().unwrap().constitutions = Some(arc_constitutions);
    
    let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
    let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
    weak_common.upgrade().unwrap().lock().unwrap().resources = Some(arc_resources.clone());

    let task = crate::roster::crews::TaskManager::new();
    let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(task));
    weak_common.upgrade().unwrap().lock().unwrap().tasks = Some(arc_tasks);
    
    let receiver = crate::communicate::receiver::NotificationReceiver::new(weak_common.clone());

    let receiver_ = receiver.clone();
    let handle = std::thread::spawn(move || {
        receiver_.init();
    });
    
    let arc_notification_receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
    weak_common.upgrade().unwrap().lock().unwrap().notification = Some(arc_notification_receiver.clone());
    
    handle.join().expect("run notification receiver failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_new_and_default() {
        let c1 = Common::new();
        assert!(c1.notification.is_none());
        assert!(c1.constitutions.is_none());
        assert!(c1.resources.is_none());
        assert!(c1.tasks.is_none());

        let c2 = Common::default();
        assert!(c2.notification.is_none());
        assert!(c2.constitutions.is_none());
        assert!(c2.resources.is_none());
        assert!(c2.tasks.is_none());
    }

    #[test]
    fn test_common_setup_components() {
        let common = std::sync::Arc::new(std::sync::Mutex::new(Common::new()));
        let weak_common = std::sync::Arc::downgrade(&common);

        let constitutions = crate::roster::crews::ConstitutionList::new(weak_common.clone());
        let arc_constitutions = std::sync::Arc::new(std::sync::Mutex::new(constitutions));
        common.lock().unwrap().constitutions = Some(arc_constitutions.clone());

        let resources = crate::roster::crews::ResourceList::new(weak_common.clone());
        let arc_resources = std::sync::Arc::new(std::sync::Mutex::new(resources));
        common.lock().unwrap().resources = Some(arc_resources.clone());

        let tasks = crate::roster::crews::TaskManager::new();
        let arc_tasks = std::sync::Arc::new(std::sync::Mutex::new(tasks));
        common.lock().unwrap().tasks = Some(arc_tasks.clone());

        let receiver = crate::communicate::receiver::NotificationReceiver::new(weak_common.clone());
        let arc_receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));
        common.lock().unwrap().notification = Some(arc_receiver.clone());

        // Verify all components accessible
        let guard = common.lock().unwrap();
        assert!(guard.constitutions.is_some());
        assert!(guard.resources.is_some());
        assert!(guard.tasks.is_some());
        assert!(guard.notification.is_some());
    }
}