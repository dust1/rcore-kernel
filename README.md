# rcore-kernel
Unix-like kernel based on RISC-V architecture written according to rCore-Tutorial-Book 


## 安装工具

### 安装binutils工具集
```
cargo install cargo-binutils

rustup component add llvm-tools-preview
```


## 不同远程仓库推送

1. 通过`git remote add [alias] [url]`命令来添加额外的远程仓库
2. 通过`git remote -v`检查是否添加成功
3. 通过`git push [alias] [branch]`来将代码推送到指定的远程仓库