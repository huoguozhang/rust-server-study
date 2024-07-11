// 引入axum框架中的各种模块和函数
use axum::{
    extract::{Json, Path, Query, State}, // 用于提取请求中的数据
    http::StatusCode, // HTTP状态码
    response::IntoResponse, // 响应转换
    routing::{get, post}, // 路由处理
    Router, // 路由器
};

// 引入bb8连接池和bb8_postgres连接管理器
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;

// 引入serde库用于序列化和反序列化
use serde::{Deserialize, Serialize};

// 引入tokio_postgres库用于与Postgres数据库交互
use tokio_postgres::NoTls;

// 引入tower_http中的TraceLayer用于日志追踪
use tower_http::trace::TraceLayer;

// 引入uuid库生成唯一标识符
use uuid::Uuid;

// 定义数据库连接池类型
type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

#[tokio::main] // 声明异步主函数
async fn main() {
    // 初始化tracing_subscriber用于日志记录
    tracing_subscriber::fmt::init();

    // 创建Postgres连接管理器
    let manager = PostgresConnectionManager::new_from_stringlike(
        "host=localhost user=postgres dbname=todolist password=changeme", // 数据库连接字符串
        NoTls,
    ).unwrap(); // 处理错误

    // 构建连接池
    let pool = Pool::builder().build(manager).await.unwrap(); // 异步构建连接池并处理错误

    // 创建axum路由器
    let app = Router::new()
        .route("/todos", get(todos_list)) // 定义GET /todos路由
        .route("/todo/new", post(todo_create)) // 定义POST /todo/new路由
        .route("/todo/update", post(todo_update)) // 定义POST /todo/update路由
        .route("/todo/delete/:id", post(todo_delete)) // 定义POST /todo/delete/:id路由
        .with_state(pool); // 传递数据库连接池状态

    // 绑定到指定地址并启动服务
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap(); // 处理错误
    tracing::debug!("listening on {}", listener.local_addr().unwrap()); // 记录监听地址
    axum::serve(listener, app).await.unwrap(); // 启动服务并处理错误
}

// 定义创建待办事项的数据结构
#[derive(Debug, Deserialize)]
struct CreateTodo {
    description: String,
}

// 定义更新待办事项的数据结构
#[derive(Debug, Deserialize)]
struct UpdateTodo {
    id: String,
    description: Option<String>,
    completed: Option<bool>,
}

// 定义待办事项的数据结构
#[derive(Debug, Serialize, Clone)]
struct Todo {
    id: String,
    description: String,
    completed: bool,
}

// 定义分页查询参数的数据结构
#[derive(Debug, Deserialize, Default)]
pub struct Pagination {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

// 创建待办事项的处理函数
async fn todo_create(
    State(pool): State<ConnectionPool>, // 提取数据库连接池状态
    Json(input): Json<CreateTodo>, // 提取并解析请求体中的JSON数据
) -> Result<(StatusCode, Json<Todo>), (StatusCode, String)> {
    let todo = Todo {
        id: Uuid::new_v4().simple().to_string(), // 生成唯一标识符
        description: input.description, // 设置描述
        completed: false, // 设置为未完成
    };

    let conn = pool.get().await.map_err(internal_error)?; // 从连接池获取连接并处理错误

    let _ret = conn
        .execute(
            "insert into todo (id, description, completed) values ($1, $2, $3) returning id",
            &[&todo.id, &todo.description, &todo.completed], // 插入数据
        )
        .await
        .map_err(internal_error)?; // 处理错误

    Ok((StatusCode::CREATED, Json(todo))) // 返回创建的待办事项和状态码
}

// 更新待办事项的处理函数
async fn todo_update(
    State(pool): State<ConnectionPool>, // 提取数据库连接池状态
    Json(utodo): Json<UpdateTodo>, // 提取并解析请求体中的JSON数据
) -> Result<(StatusCode, Json<String>), (StatusCode, String)> {
    Ok((StatusCode::OK, Json(utodo.id))) // 返回状态码和待办事项ID
}

// 删除待办事项的处理函数
async fn todo_delete(
    Path(id): Path<String>, // 提取路径参数中的ID
    State(pool): State<ConnectionPool>, // 提取数据库连接池状态
) -> Result<(StatusCode, Json<String>), (StatusCode, String)> {
    Ok((StatusCode::OK, Json(id))) // 返回状态码和待办事项ID
}

// 列出所有待办事项的处理函数
async fn todos_list(
    pagination: Option<Query<Pagination>>, // 提取查询参数
    State(pool): State<ConnectionPool>, // 提取数据库连接池状态
) -> Result<Json<Vec<Todo>>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?; // 从连接池获取连接并处理错误
    let Query(pagination) = pagination.unwrap_or_default(); // 获取分页参数
    let offset: i64 = pagination.offset.unwrap_or(0); // 设置偏移量
    let limit: i64 = pagination.limit.unwrap_or(100); // 设置限制

    let rows = conn
        .query(
            "select id, description, completed from todo offset $1 limit $2",
            &[&offset, &limit], // 查询数据
        )
        .await
        .map_err(internal_error)?; // 处理错误

    println!("rows:{:?}", rows); // 打印查询结果

    let mut todos: Vec<Todo> = Vec::new(); // 创建待办事项向量
    for row in rows {
        let id = row.get(0); // 获取ID
        let description = row.get(1); // 获取描述
        let completed = row.get(2); // 获取完成状态
        let todo = Todo {
            id,
            description,
            completed,
        }; // 创建待办事项
        todos.push(todo); // 添加到向量
    }

    Ok(Json(todos)) // 返回待办事项向量
}

// 内部错误处理函数
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()) // 返回内部服务器错误和错误信息
}
