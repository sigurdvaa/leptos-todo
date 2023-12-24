use crate::error_template::{AppError, ErrorTemplate};
use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
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
                        done BOOLEAN DEFAULT false,
                        task TEXT NOT NULL
                    );",
                ).execute(&pool).await?;
            }

            Ok(pool)
        }
    }
}

#[server(GetTodos, "/api")]
pub async fn get_todos(search: String) -> Result<Vec<TodoItem>, ServerFnError> {
    let pool = db().await?;

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let todos = match search.as_str() {
        "" => {
            sqlx::query_as::<_, TodoItem>("SELECT * FROM todos")
                .fetch_all(&pool)
                .await?
        }
        _ => {
            let search = format!("%{search}%");
            sqlx::query_as::<_, TodoItem>("SELECT * FROM todos WHERE task LIKE $1")
                .bind(search)
                .fetch_all(&pool)
                .await?
        }
    };

    Ok(todos)
}

#[server(AddTodo, "/api")]
pub async fn add_todo(todo: String) -> Result<TodoItem, ServerFnError> {
    let pool = db().await?;

    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1000));

    match sqlx::query_as::<_, TodoItem>(
        "INSERT INTO todos (task, done) VALUES ($1, false) RETURNING *",
    )
    .bind(todo)
    .fetch_one(&pool)
    .await
    {
        Ok(todo) => Ok(todo),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[server(DeleteTodo, "/api")]
pub async fn delete_todo(id: u32) -> Result<(), ServerFnError> {
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

#[server(DeleteAll, "/api")]
pub async fn delete_all() -> Result<(), ServerFnError> {
    let pool = db().await?;

    match sqlx::query("DELETE FROM todos").execute(&pool).await {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[server(ToggleTodo, "/api")]
pub async fn toggle_todo(id: u32) -> Result<TodoItem, ServerFnError> {
    let pool = db().await?;

    match sqlx::query_as::<_, TodoItem>(
        "UPDATE todos SET done = (CASE WHEN done = false THEN true ELSE false END) WHERE id = ? RETURNING *",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    {
        Ok(todo) => Ok(todo),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[server(MarkAllDone, "/api")]
pub async fn mark_all_done() -> Result<(), ServerFnError> {
    let pool = db().await?;

    match sqlx::query("UPDATE todos SET done = true")
        .execute(&pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[server(MarkAllUndone, "/api")]
pub async fn mark_all_undone() -> Result<(), ServerFnError> {
    let pool = db().await?;

    match sqlx::query("UPDATE todos SET done = false")
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
        <Stylesheet href="/css/bootstrap-icons.min.css"/>
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
    // get existing todos from server
    let existing_todos = create_resource(|| (), |_| async move { get_todos("".to_string()).await });

    // Submit changes to server
    let add_todo = create_server_action::<AddTodo>();
    let delete_todo = create_server_action::<DeleteTodo>();
    let toggle_todo = create_server_action::<ToggleTodo>();
    let mark_all_done = create_server_action::<MarkAllDone>();
    let mark_all_undone = create_server_action::<MarkAllUndone>();
    let delete_all = create_server_action::<DeleteAll>();

    // Local interaction
    let (search, set_search) = create_signal("".to_string());

    // list of todos is loaded from the server in reaction to changes
    //let todos = create_resource(move || search.get(), get_todos);

    // Merge todos from changes and interactions into one signal
    let (todos, set_todos) = create_signal::<Vec<RwSignal<TodoItem>>>(vec![]);
    create_effect(move |_| {
        if let Some(Ok(exiting_todos)) = existing_todos.get() {
            set_todos.update(|todos| {
                todos.extend(exiting_todos.into_iter().map(|todo| create_rw_signal(todo)))
            });
        }
    });
    create_effect(move |_| {
        if let Some(Ok(todo)) = add_todo.value().get() {
            set_todos.update(|todos| todos.push(create_rw_signal(todo)));
        };
    });
    create_effect(move |_| {
        if let Some(Ok(toggled)) = toggle_todo.value().get() {
            set_todos.update(|todos| {
                for todo in todos {
                    if todo.get().id == toggled.id {
                        todo.set(toggled);
                        logging::log!("updated todo");
                        break;
                    }
                }
            });
        };
    });
    /*
        let todos = Signal::derive(move || {
            if let Some(Ok(exiting_todos)) = exiting_todos.get() {
                set_todos.update(|todos| {
                    todos.extend(exiting_todos.into_iter().map(|todo| create_rw_signal(todo)))
                });
            }

            match add_todo.value().get() {
                Some(Ok(todo)) => set_todos.update(|todos| todos.push(create_rw_signal(todo))),
                _ => (),
            };
            get_todos()
                .into_iter()
                .filter(|todo: &RwSignal<TodoItem>| todo.get().task.contains(&search()))
                .collect::<Vec<RwSignal<TodoItem>>>()
        });
    */
    view! {
        <Topbar set_search/>
        <div class="container mt-3">
            <AllTodosAction mark_all_done mark_all_undone delete_all/>
        </div>
        <div class="container mt-3">
            <Todoadd add_todo />
        </div>
        <div class="container mt-3">
            <Todolist todos delete_todo toggle_todo/>
        </div>
    }
}

#[component]
fn Topbar(set_search: WriteSignal<String>) -> impl IntoView {
    view! {
        <nav class="navbar navbar-expand-md" style="background-color: #301934">
            <div class="container-fluid">
                <a class="navbar-brand" href="#"><i class="bi bi-card-checklist text-warning me-1"></i> Todo</a>
                <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarSupportedContent"
                    aria-controls="navbarSupportedContent" aria-expanded="false" aria-label="Toggle navigation">
                <span class="navbar-toggler-icon"></span>
                </button>
                <div class="collapse navbar-collapse" id="navbarSupportedContent">
                    <ul class="navbar-nav me-auto mb-2 mb-lg-0">
                    </ul>
                    <div class="d-flex" role="search">
                        <div class="input-group flex-nowrap">
                            <span class="input-group-text" id="addon-wrapping">
                               <i class="bi bi-search"></i>
                            </span>
                            <input class="form-control me-2" type="search" placeholder="Search Todos" aria-label="Search"
                                prop:value=""
                                on:change=move |ev| set_search.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                </div>
            </div>
        </nav>
    }
}

#[component]
fn Todolist(
    //todos: Resource<String, Result<Vec<TodoItem>, ServerFnError>>,
    todos: ReadSignal<Vec<RwSignal<TodoItem>>>,
    delete_todo: Action<DeleteTodo, Result<(), leptos::ServerFnError>>,
    toggle_todo: Action<ToggleTodo, Result<TodoItem, leptos::ServerFnError>>,
) -> impl IntoView {
    view! {
        <div>
            <Suspense fallback=move || view! { <p class="text-muted">"Loading..."</p> }>
            /*
                {move || match todos() {
                    None => view! { <p class="text-muted">"No data"</p> }.into_view(),
                    Some(result) => match result {
                        Err(e) => view! { <p class="text-danger">"Error loading: " {e.to_string()}</p> }.into_view(),
                        Ok(data) => view! { <ShowTodos data delete_todo toggle_todo/> }.into_view(),
                    }
                }}
            */
            <ShowTodos todos delete_todo toggle_todo/>
            </Suspense>
        </div>
    }
}

#[component]
fn ShowTodos(
    todos: ReadSignal<Vec<RwSignal<TodoItem>>>,
    delete_todo: Action<DeleteTodo, Result<(), leptos::ServerFnError>>,
    toggle_todo: Action<ToggleTodo, Result<TodoItem, leptos::ServerFnError>>,
) -> impl IntoView {
    view! {
        <For
            // a function that returns the items we're iterating over; a signal is fine
            each=move || todos.get()
            // a unique key for each item
            key=|item| item.get().id
            // renders each item to a view
            children=move |item| {
                let item = item.get();
                let toggle_class = format!("btn btn-sm border-0 bi {}",
                    if item.done {
                        "bi-check-square-fill btn-outline-success"
                    } else {
                        "bi-square btn-outline-warning"
                    });
                view! {
                    <div class="card mt-3" style="background-color: #301934">
                        <div class="card-body d-flex">
                            <div>
                                <ActionForm action=toggle_todo>
                                    <input type="hidden" name="id" value={item.id}/>
                                    <button type="submit" value="" class={toggle_class}/>
                                </ActionForm>
                            </div>
                            <div class="flex-fill text-start mx-3">
                                {item.task}
                            </div>
                            <div class="ms-auto">
                                <ActionForm action=delete_todo>
                                    <input type="hidden" name="id" value={item.id}/>
                                    <button type="submit" value="" class="btn btn-sm border-0 btn-outline-danger bi bi-trash-fill"/>
                                </ActionForm>
                            </div>
                        </div>
                    </div>
                }
            }
        />
    }
}

#[component]
fn Todoadd(add_todo: Action<AddTodo, Result<TodoItem, leptos::ServerFnError>>) -> impl IntoView {
    view! {
        <ActionForm action=add_todo>
            <div class="input-group">
                <div class="form-floating">
                    <input type="text" name="todo" id="floatingTodo" class="form-control"
                        placeholder="Take out the trash" required autofocus
                        readonly=move || add_todo.pending().get()
                        prop:value=move || match add_todo.input().get() {
                            Some(value) => value.todo,
                            None => "".into(),
                        }
                    />
                    <label for="floatingTodo" class="text-muted">New todo...</label>
                </div>
                <button type="submit" class="btn btn-outline-success col-lg-1" disabled=move || add_todo.pending().get()>
                    <span hidden=move || add_todo.pending().get()>+ Add</span>
                    <div hidden=move || !add_todo.pending().get() class="spinner-border spinner-border-sm" role="status"></div>
                </button>
            </div>
        </ActionForm>
    }
}

#[component]
fn AllTodosAction(
    mark_all_done: Action<MarkAllDone, Result<(), leptos::ServerFnError>>,
    mark_all_undone: Action<MarkAllUndone, Result<(), leptos::ServerFnError>>,
    delete_all: Action<DeleteAll, Result<(), leptos::ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="d-flex justify-content-center">
            <ActionForm action=mark_all_done>
                <input type="submit" value="All Done" class="btn btn-outline-success mx-3"/>
            </ActionForm>
            <ActionForm action=mark_all_undone>
                <input type="submit" value="All Undone" class="btn btn-outline-warning mx-3"/>
            </ActionForm>
            <input type="button" value="Delete All" class="btn btn-outline-danger mx-3" data-bs-toggle="modal" data-bs-target="#confirm-delete"/>
        </div>

        <div class="modal" tabindex="-1" id="confirm-delete">
            <div class="modal-dialog">
                <div class="modal-content">
                    <div class="modal-header">
                        <h5 class="modal-title text-danger">Delete All</h5>
                        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                    </div>
                    <div class="modal-body text-start">
                        <p>This will delete all todos, are you sure?</p>
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Close</button>
                        <ActionForm action=delete_all>
                            <input type="submit" value="Delete All" class="btn btn-danger" data-bs-dismiss="modal"/>
                        </ActionForm>
                    </div>
                </div>
            </div>
        </div>
    }
}
