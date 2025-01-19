use axum::{
    extract::State,
    response::{Html, Redirect},
    Form,
};
use serde::Deserialize;

use crate::{todo::Todo, AppState};

const STYLES: &str = include_str!("styles.css");

pub async fn list_todos(State(state): State<AppState>) -> Html<String> {
    let todos = state.todos.lock().unwrap();
    let mut html = format!(
        r#"
        <html>
            <head>
                <script src="https://unpkg.com/htmx.org@1.9.10"></script>
                <style>
                    {styles}
                </style>
            </head>
            <body hx-boost="true">
                <h1>Todo List</h1>
                <form action="/todo" method="post" autocomplete="off">
                    <input type="text" name="description" placeholder="New todo" autocomplete="off" autofocus="true" required>
                    <button type="submit">Add Todo</button>
                </form>
                <ul>
        "#,
        styles = STYLES
    );

    for todo in todos.iter() {
        html.push_str(&format!(
            r#"
            <li>
                <form style="display: inline; margin: 0;" action="/todo/{}/toggle" method="post">
                    <button type="submit" class="checkbox-button {}">
                        {}
                    </button>
                </form>
                <span class="todo-text" style="{}">
                    {}
                </span>
                <form style="display: inline; margin: 0;" action="/todo/{}/delete" method="post">
                    <button class="delete-btn" type="submit">🗑️</button>
                </form>
            </li>
            "#,
            todo.id,
            if todo.completed { "checked" } else { "" },
            if todo.completed { "✓" } else { "" },
            if todo.completed {
                "text-decoration: line-through; color: #888;"
            } else {
                ""
            },
            todo.description,
            todo.id
        ));
    }

    html.push_str("</ul></body></html>");
    Html(html)
}

#[derive(Deserialize)]
pub struct NewTodo {
    description: String,
}

pub async fn create_todo(State(state): State<AppState>, Form(todo): Form<NewTodo>) -> Redirect {
    let mut todos = state.todos.lock().unwrap();

    let next_id = todos
        .iter()
        .map(|todo| todo.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    todos.push(Todo {
        id: next_id,
        description: todo.description,
        completed: false,
    });

    Redirect::to("/")
}

pub async fn delete_todo(
    State(state): State<AppState>,
    id: axum::extract::Path<usize>,
) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    todos.retain(|todo| todo.id != *id);
    Redirect::to("/")
}

pub async fn toggle_todo(
    State(state): State<AppState>,
    id: axum::extract::Path<usize>,
) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if let Some(todo) = todos.iter_mut().find(|t| t.id == *id) {
        todo.completed = !todo.completed;
    }
    Redirect::to("/")
}
