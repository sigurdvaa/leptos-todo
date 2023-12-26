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
pub async fn get_todos() -> Result<Vec<TodoItem>, ServerFnError> {
    let pool = db().await?;

    // fake API delay
    // std::thread::sleep(std::time::Duration::from_millis(1000));

    let todos = sqlx::query_as::<_, TodoItem>("SELECT * FROM todos")
        .fetch_all(&pool)
        .await?;

    Ok(todos)
}

#[server(AddTodo, "/api")]
pub async fn add_todo(todo: String) -> Result<TodoItem, ServerFnError> {
    let pool = db().await?;

    // fake API delay
    // std::thread::sleep(std::time::Duration::from_millis(1000));

    match sqlx::query_as::<_, TodoItem>(
        "INSERT INTO todos (task, done) VALUES (?, false) RETURNING *",
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
pub async fn delete_todo(id: u32) -> Result<u32, ServerFnError> {
    let pool = db().await?;

    match sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
    {
        Ok(_) => Ok(id),
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
pub async fn toggle_todo(id: u32) -> Result<u32, ServerFnError> {
    let pool = db().await?;

    match sqlx::query(
        "UPDATE todos SET done = (CASE WHEN done = false THEN true ELSE false END) WHERE id = ?",
    )
    .bind(id)
    .execute(&pool)
    .await
    {
        Ok(_) => Ok(id),
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
    // filter input
    let filter = create_rw_signal(String::new());

    // list of todos
    let todos = create_rw_signal::<Vec<RwSignal<TodoItem>>>(vec![]);

    // get existing and create inital todo list
    // let existing_todos = create_resource(|| (), |_| async move { get_todos().await });
    let get_todos = create_server_action::<GetTodos>();
    get_todos.dispatch(GetTodos {});
    create_effect(move |_| {
        logging::log!("running effect for get_todos");
        if let Some(Ok(existing_todos)) = get_todos.value().get() {
            todos.update(|todos| {
                todos.extend(
                    existing_todos
                        .into_iter()
                        .map(|todo| create_rw_signal(todo)),
                )
            });
        }
    });

    // add
    let add_todo = create_server_action::<AddTodo>();
    create_effect(move |_| {
        logging::log!("running effect for add_todo");
        if let Some(Ok(todo)) = add_todo.value().get() {
            todos.update(|todos| todos.push(create_rw_signal(todo)));
        };
    });

    // toggle
    let toggle_todo = create_server_action::<ToggleTodo>();
    create_effect(move |_| {
        logging::log!("running effect for toggle_todo");
        if let Some(Ok(toggled_id)) = toggle_todo.value().get() {
            todos.with_untracked(|todos| {
                for todo in todos.iter() {
                    if todo.with_untracked(|todo| todo.id == toggled_id) {
                        todo.update(|todo| todo.done = !todo.done);
                        break;
                    }
                }
            });
        };
    });

    // delete
    let delete_todo = create_server_action::<DeleteTodo>();
    create_effect(move |_| {
        logging::log!("running effect for delete_todo");
        if let Some(Ok(del_id)) = delete_todo.value().get() {
            todos
                .update(|todos| todos.retain(|todo| todo.with_untracked(|todo| todo.id != del_id)));
        };
    });

    // all done
    let mark_all_done = create_server_action::<MarkAllDone>();
    create_effect(move |_| {
        logging::log!("running effect for mark_all_done");
        if let Some(Ok(())) = mark_all_done.value().get() {
            todos.with_untracked(|todos| {
                todos
                    .iter()
                    .for_each(|todo| todo.update(|todo| todo.done = true))
            });
        };
    });

    // all undone
    let mark_all_undone = create_server_action::<MarkAllUndone>();
    create_effect(move |_| {
        logging::log!("running effect for mark_all_undone");
        if let Some(Ok(())) = mark_all_undone.value().get() {
            todos.with_untracked(|todos| {
                todos
                    .iter()
                    .for_each(|todo| todo.update(|todo| todo.done = false))
            });
        };
    });

    // all delete
    let delete_all = create_server_action::<DeleteAll>();
    create_effect(move |_| {
        logging::log!("running effect for delete_all");
        if let Some(Ok(())) = delete_all.value().get() {
            todos.update(|todos| todos.clear());
        };
    });

    view! {
        <Topbar filter/>
        <div class="container mt-3">
            <AllTodosAction mark_all_done mark_all_undone delete_all/>
        </div>
        <div class="container mt-3">
            <Todoadd add_todo />
        </div>
        <div class="container mt-3">
            /*
            {move || { match todos.with(|todos| todos.is_empty()) {
                true => view! {<p class="text-muted">No tasks!</p>}.into_view(),
                false => view! {<Todolist todos delete_todo toggle_todo filter/>}.into_view(),
            }}}
            */
            <Todolist todos delete_todo toggle_todo filter/>
        </div>
    }
}

#[component]
fn Topbar(filter: RwSignal<String>) -> impl IntoView {
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
                            <input class="form-control me-2" type="search" placeholder="Filter"
                                prop:value=""
                                on:input=move |ev| filter.set(event_target_value(&ev))
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
    todos: RwSignal<Vec<RwSignal<TodoItem>>>,
    delete_todo: Action<DeleteTodo, Result<u32, leptos::ServerFnError>>,
    toggle_todo: Action<ToggleTodo, Result<u32, leptos::ServerFnError>>,
    filter: RwSignal<String>,
) -> impl IntoView {
    let card_class = move |todo: RwSignal<TodoItem>| {
        let mut class = "card mt-3".to_string();
        if !todo.with(|todo| todo.task.contains(&filter.get())) {
            class.push_str(" visually-hidden");
        }
        class
    };
    let toggle_class = move |todo: RwSignal<TodoItem>| {
        format!(
            "btn btn-sm border-0 bi {}",
            if todo.with(|todo| todo.done) {
                "bi-check-square-fill btn-outline-success"
            } else {
                "bi-square btn-outline-warning"
            }
        )
    };
    view! {<For
        each=todos
        key=|todo| todo.with(|todo| todo.id)
        children=move |todo| {
            view! {
                <div class={move || card_class(todo)} style="background-color: #301934">
                    <div class="card-body d-flex">
                        <div>
                            <ActionForm action=toggle_todo>
                                <input type="hidden" name="id" value={move || todo.with(|todo| todo.id)}/>
                                <button type="submit" value="" class={move || toggle_class(todo)}/>
                            </ActionForm>
                        </div>
                        <div class="flex-fill text-start mx-3">
                            {move || todo.with(|todo| todo.task.clone())}
                        </div>
                        <div class="ms-auto">
                            <ActionForm action=delete_todo>
                                <input type="hidden" name="id" value={move || todo.with(|todo| todo.id)}/>
                                <button type="submit" value="" class="btn btn-sm border-0 btn-outline-danger bi bi-trash-fill"/>
                            </ActionForm>
                        </div>
                    </div>
                </div>
            }
        }
    />}
}

#[component]
fn Todoadd(add_todo: Action<AddTodo, Result<TodoItem, leptos::ServerFnError>>) -> impl IntoView {
    view! {
        <ActionForm action=add_todo>
            <div class="input-group">
                <div class="form-floating" class:placeholder-glow=move || add_todo.pending().get()>
                    <input type="text" name="todo" id="floatingTodo" class="form-control"
                        class:placeholder=move || add_todo.pending().get()
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
