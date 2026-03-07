# mp4_smaller_rust
项目仓库地址：[https://github.com/MulsysInArk/mp4_smaller_rust.git](https://github.com/MulsysInArk/mp4_smaller_rust.git "mp4_smaller_rust项目地址") 
## 背景
由于QQ群里老是有群友发个几百MB的视频，大家都不愿意看。
有时候
1. 录集锦体积也挺大的，但是又有人发20多分钟的猫和老鼠，才10MB。
2. 网络上难找到在线极限压缩视频体积的网站
所以我使用RUST+ffmpeg使用cursor写了个二次压缩的程序，并且打包成了exe可以直接使用。
## 效果
64M->1.18M 230M->60M，画质和音质会有些许失真，但是在手机上浏览影响不大
## 介绍
mp4_smaller_rust  
**功能**： 本程序基于ffmpeg，ffprobe将大体积的mp4文件强行压缩到小体积，例如：64M->1.18M 230M->60M  
**环境**： 若使用源码，需要先下载ffmpeg，ffprobe并将.\bin文件夹加载在系统环境变量 在cmd中输入ffmepg，ffprobe验证安装是否成功  
           使用打包的exe文件不需要别的环境条件。  
**使用方法**： 使用.exe，将input.mp4和mp4_shrink.exe放在一个文件夹 
```shell
.\mp4_shrink.exe input.mp4 output.mp4 --target-bytes 10000000  
```

使用rust源码，将input.mp4放在mp4_smaller_rust文件夹 
```shell
cargo run --release -- input.mp4 output.mp4 --target-bytes 100000000
```



