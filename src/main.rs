use eframe::egui;
use rusqlite::{params, Connection, Result};

#[derive(Debug, Clone)]
struct TodoItem {
    id: i32,
    title: String,
    description: Option<String>,
    done: bool,
    deleted: bool,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Filter {
    All,
    Active,
    Completed,
    Deleted,
}

struct TodoApp {
    conn: Connection,
    todos: Vec<TodoItem>,
    new_title: String,
    new_description: String,
    edit_todo_id: Option<i32>,
    edit_title: String,
    edit_description: String,
    filter: Filter,
}

impl TodoApp {
    /// Initialize the app: open DB, create table, and load todos.
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let conn = Connection::open("todos.db").expect("Failed to open DB");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                description TEXT,
                done BOOLEAN NOT NULL,
                deleted BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        )
        .expect("Failed to create table");

        let mut app = Self {
            conn,
            todos: Vec::new(),
            new_title: String::new(),
            new_description: String::new(),
            edit_todo_id: None,
            edit_title: String::new(),
            edit_description: String::new(),
            filter: Filter::All,
        };
        app.load_todos();
        app
    }

    /// Load all todos from SQLite.
    fn load_todos(&mut self) {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, description, done, deleted FROM todos")
            .expect("Failed to prepare query");
        let rows = stmt
            .query_map([], |row| {
                Ok(TodoItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    done: row.get(3)?,
                    deleted: row.get(4)?,
                })
            })
            .expect("Failed to query todos");

        self.todos = rows.map(|r| r.unwrap()).collect();
    }

    /// Insert a new todo.
    fn add_todo(&mut self) {
        let title = self.new_title.trim();
        if !title.is_empty() {
            let desc = self.new_description.trim();
            let desc_opt = if desc.is_empty() { None } else { Some(desc) };
            self.conn
                .execute(
                    "INSERT INTO todos (title, description, done, deleted) VALUES (?1, ?2, 0, 0)",
                    params![title, desc_opt],
                )
                .expect("Failed to insert todo");
            self.new_title.clear();
            self.new_description.clear();
            self.load_todos();
        }
    }

    /// Toggle completion state.
    fn toggle_done(&mut self, id: i32, current: bool) {
        self.conn
            .execute(
                "UPDATE todos SET done = ?1 WHERE id = ?2",
                params![!current, id],
            )
            .expect("Failed to update todo");
        self.load_todos();
    }

    /// Mark a todo as deleted (soft delete).
    fn delete_todo(&mut self, id: i32) {
        self.conn
            .execute(
                "UPDATE todos SET deleted = 1 WHERE id = ?1",
                params![id],
            )
            .expect("Failed to mark deleted");
        self.load_todos();
    }

    /// Restore a soft-deleted todo.
    fn restore_todo(&mut self, id: i32) {
        self.conn
            .execute(
                "UPDATE todos SET deleted = 0 WHERE id = ?1",
                params![id],
            )
            .expect("Failed to restore todo");
        self.load_todos();
    }

    /// Update the title and description of an existing todo using internal fields.
    fn update_todo(&mut self, id: i32) {
        let t = self.edit_title.trim();
        if t.is_empty() {
            return;
        }
        let d = self.edit_description.trim();
        let d_opt = if d.is_empty() { None } else { Some(d) };
        self.conn
            .execute(
                "UPDATE todos SET title = ?1, description = ?2 WHERE id = ?3",
                params![t, d_opt, id],
            )
            .expect("Failed to update todo");
        self.load_todos();
    }

    /// Return todos filtered by the current `filter` setting.
    fn filtered_todos(&self) -> Vec<TodoItem> {
        self.todos
            .iter()
            .cloned()
            .filter(|t| match self.filter {
                Filter::All => !t.deleted,
                Filter::Active => !t.done && !t.deleted,
                Filter::Completed => t.done && !t.deleted,
                Filter::Deleted => t.deleted,
            })
            .collect()
    }
}

impl eframe::App for TodoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🚀 TODO List");

            // Add new todo: title + optional description
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_title)
                        .hint_text("Title"),
                );
                ui.add(
                    egui::TextEdit::multiline(&mut self.new_description)
                        .desired_rows(1)
                        .hint_text("Description (optional)"),
                );
                if ui.button("Add").clicked() {
                    self.add_todo();
                }
            });

            ui.separator();

            // Filter selector
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.filter, Filter::All, "All");
                ui.selectable_value(&mut self.filter, Filter::Active, "Active");
                ui.selectable_value(&mut self.filter, Filter::Completed, "Completed");
                ui.selectable_value(&mut self.filter, Filter::Deleted, "Deleted");
            });

            ui.separator();

            // Display and interact with todos
            for todo in self.filtered_todos() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Checkbox (disabled for deleted)
                        if !todo.deleted {
                            let mut done = todo.done;
                            if ui.checkbox(&mut done, "").clicked() {
                                self.toggle_done(todo.id, todo.done);
                            }
                        } else {
                            ui.add_enabled(false, egui::Checkbox::new(&mut false, ""));
                        }

                        // Title and description display
                        ui.vertical(|ui| {
                            ui.label(&todo.title);
                            if let Some(desc) = &todo.description {
                                ui.label(desc);
                            }
                        });

                        // Actions: edit/delete or restore
                        if Some(todo.id) == self.edit_todo_id {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.edit_title)
                                    .hint_text("Title"),
                            );
                            ui.add(
                                egui::TextEdit::multiline(&mut self.edit_description)
                                    .desired_rows(1),
                            );
                            if ui.button("Save").clicked() {
                                self.update_todo(todo.id);
                                self.edit_todo_id = None;
                            }
                            if ui.button("Cancel").clicked() {
                                self.edit_todo_id = None;
                            }
                        } else if !todo.deleted {
                            if ui.small_button("✏️").clicked() {
                                self.edit_todo_id = Some(todo.id);
                                self.edit_title = todo.title.clone();
                                self.edit_description = todo.description.clone().unwrap_or_default();
                            }
                            if ui.small_button("🗑️").clicked() {
                                self.delete_todo(todo.id);
                            }
                        } else {
                            if ui.small_button("↩️").clicked() {
                                self.restore_todo(todo.id);
                            }
                        }
                    });
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "TODO App",
        options,
        Box::new(|cc| Ok(Box::new(TodoApp::new(cc)))),
    )?;
    Ok(())
}

