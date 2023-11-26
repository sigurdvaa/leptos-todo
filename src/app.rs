use crate::error_template::{AppError, ErrorTemplate};
use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    id: u32,
    done: bool,
    task: String,
}

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

        pub async fn db() -> Result<SqlitePool, ServerFnError> {
            let filename = "Todos.db";
            let mut created = false;
            if !Sqlite::database_exists(&filename).await? {
                Sqlite::create_database(&filename).await?;
                created = true;
            }

            let pool = SqlitePool::connect(&filename).await?;

            if created {
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS todos (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        done BOOLEAN NOT NULL,
                        task TEXT NOT NULL
                    );",
                ).execute(&pool).await?;
            }

            Ok(pool)
        }
    }
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<TodoItem>, ServerFnError> {
    let pool = db().await?;

    let todos = sqlx::query_as::<_, TodoItem>("SELECT * FROM todos")
        .fetch_all(&pool)
        .await?;

    Ok(todos)
}

#[server(AddTodo, "/api")]
pub async fn add_todo(todo: String) -> Result<(), ServerFnError> {
    let pool = db().await?;

    match sqlx::query("INSERT INTO todos (task, done) VALUES ($1, false)")
        .bind(todo)
        .execute(&pool)
        .await
    {
        Ok(_row) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // allow any component to get dark mode state via context
    // let (dark_mode, _) = create_signal(true);
    // provide_context(dark_mode);

    view! {

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos-todo.css"/>

        // sets the document title
        <Title text="Todo"/>

        <Meta
            name="color-scheme"
            content="dark"
        />

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let add_todo = create_server_multi_action::<AddTodo>();
    //let delete_todo = create_server_action::<DeleteTodo>();

    // list of todos is loaded from the server in reaction to changes
    let todos = create_resource(move || (add_todo.version().get()), move |_| get_todos());

    view! {
        <Sidebar />
        <Todoadd add_todo/>
        <Todolist todos/>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <h1>Todo</h1>
    }
}

#[component]
fn Todolist(todos: Resource<usize, Result<Vec<TodoItem>, ServerFnError>>) -> impl IntoView {
    view! {
        <div>
            <Suspense fallback=move || view! { <p>"Loading (Suspense Fallback)..."</p> }>
                {move || match todos.get() {
                    None => view! { <p>"Loading... (no data)"</p> }.into_view(),
                    Some(result) => match result {
                        Err(e) => view! { <p>"Error loading: " {e.to_string()}</p> }.into_view(),
                        Ok(data) => view! { <ShowTodos data /> }.into_view(),
                    }
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn ShowTodos(data: Vec<TodoItem>) -> impl IntoView {
    view! {
        <For
            // a function that returns the items we're iterating over; a signal is fine
            each=move || data.clone().into_iter()
            // a unique key for each item
            key=|item| item.id
            // renders each item to a view
            children=move |item| {
                view! {
                    <div>{if item.done {"D"} else {"U"}} " " {item.task}</div>
                }
            }
        />
    }
}

#[component]
fn Todoadd(add_todo: MultiAction<AddTodo, Result<(), leptos::ServerFnError>>) -> impl IntoView {
    view! {
        <MultiActionForm action=add_todo>
            <label>
                "Add a Todo"
                <input type="text" name="todo"/>
            </label>
            <input type="submit" value="Add"/>
        </MultiActionForm>
    }
}
