
1. 构建
1> 单个构建或者指定构建单独的crate
    cargo build --bin=crew --package=crew
2> 构建所有
    cargo build --workspace

2. 运行
   1.先启动captain.exe
   2.再启动crew.exe， crew.exe 启动之后会默认启动cocrew.exe，如果仅仅想运行crew.exe, 则需要手动传参给noco 
