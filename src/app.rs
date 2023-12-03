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

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1000));

    match sqlx::query("INSERT INTO todos (task, done) VALUES ($1, false)")
        .bind(todo)
        .execute(&pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[server(DeleteTodo, "/api")]
pub async fn del_todo(id: u32) -> Result<(), ServerFnError> {
    let pool = db().await?;

    match sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Html attr:data-bs-theme="dark" />

        // Bootstrap
        <Stylesheet href="/css/bootstrap.min.css"/>
        <Script src="/js/bootstrap.bundle.min.js" defer="true"/>

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos-todo.css"/>

        // sets the document title
        <Title text="Todo"/>

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
    let delete_todo = create_server_action::<DeleteTodo>();

    // list of todos is loaded from the server in reaction to changes
    let todos = create_resource(
        move || (add_todo.version().get(), delete_todo.version().get()),
        move |_| get_todos(),
    );

    view! {
        <Sidebar />
        <Todoadd add_todo/>
        <Todolist todos delete_todo/>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <nav class="navbar navbar-expand-lg bg-body-tertiary">
          <div class="container-fluid">
            <a class="navbar-brand" href="#">Todo</a>
            <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarSupportedContent" aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
              <span class="navbar-toggler-icon"></span>
            </button>
            <div class="collapse navbar-collapse" id="navbarSupportedContent">
              <ul class="navbar-nav me-auto mb-2 mb-lg-0">
                <li class="nav-item">
                  <a class="nav-link" aria-current="page" href="#">Mark All Done</a>
                </li>
                <li class="nav-item">
                  <a class="nav-link" href="#">Mark All Undone</a>
                </li>
                <li class="nav-item">
                  <a class="nav-link disabled text-danger" aria-disabled="true">Delete All</a>
                </li>
              </ul>
              <form class="d-flex" role="search">
                <input class="form-control me-2" type="search" placeholder="Search" aria-label="Search"/>
                <button class="btn btn-outline-success" type="submit">Search</button>
              </form>
            </div>
          </div>
        </nav>
    }
}

#[component]
fn Todolist(
    todos: Resource<(usize, usize), Result<Vec<TodoItem>, ServerFnError>>,
    delete_todo: Action<DeleteTodo, Result<(), leptos::ServerFnError>>,
) -> impl IntoView {
    view! {
        <div>
            <Transition fallback=move || view! { <p class="text-muted">"Loading..."</p> }>
                {move || match todos.get() {
                    None => view! { <p class="text-muted">"No data"</p> }.into_view(),
                    Some(result) => match result {
                        Err(e) => view! { <p class="text-danger">"Error loading: " {e.to_string()}</p> }.into_view(),
                        Ok(data) => view! { <ShowTodos data delete_todo/> }.into_view(),
                    }
                }}
            </Transition>
        </div>
    }
}

#[component]
fn ShowTodos(
    data: Vec<TodoItem>,
    delete_todo: Action<DeleteTodo, Result<(), leptos::ServerFnError>>,
) -> impl IntoView {
    view! {
        <For
            // a function that returns the items we're iterating over; a signal is fine
            each=move || data.clone().into_iter()
            // a unique key for each item
            key=|item| item.id
            // renders each item to a view
            children=move |item| {
                view! {
                    <div>{if item.done {"D"} else {"U"}} " " {item.task}
                        <ActionForm action=delete_todo class="d-inline ps-2">
                            <input type="hidden" name="id" value={item.id}/>
                            <input type="submit" value="X" class="btn btn-sm btn-outline-warning"/>
                        </ActionForm>
                    </div>
                }
            }
        />
    }
}

#[component]
fn Todoadd(add_todo: MultiAction<AddTodo, Result<(), leptos::ServerFnError>>) -> impl IntoView {
    view! {
        <div>
            <MultiActionForm action=add_todo>
                <label>
                    "Add a Todo"
                    <input type="text" name="todo" class="form-control"/>
                </label>
                <input type="submit" value="Add" class="btn btn-outline-success"/>
            </MultiActionForm>
        </div>
    }
}
