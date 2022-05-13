import os

base_address = 0x80400000
step = 0x20000
linker = 'src/linker.ld'

app_id = 0
apps = os.listdir('src/bin')
apps.sort()
for app in apps:
    app = app[:app.find('.')]
    lines = []
    lines_before = []
    # 找到linker.ld中的BASE_ADDRESS = 0x80400000
    # 将表达式后面的地址替换为当前应用的起始地址
    with open(linker, 'r') as f:
        for line in f.readlines():
            lines_before.append(line)
            line = line.replace(hex(base_address), hex(base_address + step * app_id))
            lines.append(line)
    with open(linker, 'w+') as f:
        f.writelines(lines)
    # 构建这一个应用程序
    os.system('cargo build --bin %s --release' % app)
    dir_path = 'target/riscv64gc-unknown-none-elf/release/'
    os.system('rust-objcopy --binary-architecture=riscv64 %s --strip-all -O binary %s' % (dir_path + app, dir_path + app + '.bin'))
    print('[build.py] application %s start with address %s' %( app, hex(base_address + step * app_id)))
    # 将linker.ld中的BASE_ADDRESS表达式还原
    with open(linker, 'w+') as f:
        f.writelines(lines_before)
    app_id = app_id + 1