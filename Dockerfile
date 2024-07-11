# 使用官方的最新Rust镜像作为基础镜像
FROM rust:latest

# 安装构建工具和依赖项
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    libssl-dev \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /usr/src/rust-study

# 将Cargo.toml和Cargo.lock复制到工作目录
COPY Cargo.toml Cargo.lock ./

# 创建空的src目录，以便缓存依赖项
RUN mkdir src

# 预先编译依赖项
RUN cargo build --release
RUN rm -rf src

# 复制项目的所有源文件到工作目录
COPY . .

# 再次构建项目
RUN cargo build --release

# 暴露应用运行的端口
EXPOSE 3000

# 运行构建好的二进制文件
CMD ["./target/release/rust-study"]
