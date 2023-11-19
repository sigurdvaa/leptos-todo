use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    id: u64,
    done: bool,
    task: String,
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<TodoItem>, ServerFnError> {
    Ok(vec![
        TodoItem {
            id: 0,
            done: true,
            task: "Empty garbage".into(),
        },
        TodoItem {
            id: 1,
            done: true,
            task: "Clean toilet".into(),
        },
        TodoItem {
            id: 2,
            done: false,
            task: "Buy diapers".into(),
        },
        TodoItem {
            id: 3,
            done: false,
            task: "Walk the dog".into(),
        },
    ])
}

#[server(AddTodo, "/api")]
pub async fn add_todo(_todo: String) -> Result<(), ServerFnError> {
    Ok(())
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
        <Todolist todos/>
        //<Todoadd set_todos/>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    view! {
        <h1>Todo</h1>
    }
}

#[component]
fn Todolist(todos: Resource<usize, Result<Vec<TodoItem>, leptos::ServerFnError>>) -> impl IntoView {
    view! {
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || {
                todos.get()
                    .map(|item| view! {
                        <div>{if item.done {"D"} else {"U"}} " " {item.task}</div>
                    })
            }}
        </Suspense>

    /*
        <Suspense>
            <div>
                {move || match todos.get() {
                    None => view! { <p>"Loading..."</p> }.into_view(),
                    Some(result) => match result {
                        Err(e) => view! { <p>"Error loading: " {e.to_string()}</p> }.into_view(),
                        Ok(data) => view! { <ShowTodos data /> }.into_view(),
                    }
                }}
            </div>
        </Suspense>
    */
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
fn Todoadd(_set_todos: Action<String, Result<(), ServerFnError>>) -> impl IntoView {
    view! {
        <div><input type="text"/><button>Add</button></div>
    }
}
