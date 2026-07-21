pub fn lookup(key: &str) -> Option<&'static str> {
    match key {
        "" => Some(ROOT),
        "tasks" => Some(TASKS),
        "tasks list" => Some(TASKS_LIST),
        "tasks add" => Some(TASKS_ADD),
        "tasks update" => Some(TASKS_UPDATE),
        "tasks update status" => Some(TASKS_UPDATE_STATUS),
        "tasks update due" => Some(TASKS_UPDATE_DUE),
        "tasks update title" => Some(TASKS_UPDATE_TITLE),
        "tasks update description" => Some(TASKS_UPDATE_DESCRIPTION),
        "tasks delete" => Some(TASKS_DELETE),
        "tasks history" => Some(TASKS_HISTORY),
        _ => None,
    }
}

const ROOT: &str = "\
Jade — local-first personal toolkit (CLI)

USAGE
  jade <feature> <verb> [options]
  jade help [topic…]
  jade <topic…> help

FEATURES
  tasks    Task tracking CRUD

GLOBAL OPTIONS
  --db <path>   Use a specific SQLite database (default: app data dir)
  --json        Machine-readable JSON output where applicable
  -h, --help    Clap help for the current command
  help          Rich topic help (this system)

EXAMPLES
  jade tasks list
  jade tasks add \"Buy milk\" --due tomorrow --tag errands
  jade tasks update --id <uuid> --status active
  jade tasks delete --id <uuid>
  jade tasks history --id <uuid>

HELP TOPICS
  jade help
  jade tasks help
  jade tasks add help
  jade tasks update help
  jade tasks update status help
  jade tasks update due help
  jade tasks update title help
  jade tasks update description help
  jade tasks delete help
  jade tasks history help

NOTES
  The GUI does not need to be running. The CLI opens the same SQLite file
  as the desktop app (app.jade.desktop/jade.db under your user data dir).
";

const TASKS: &str = "\
jade tasks — task tracking

USAGE
  jade tasks <verb> [options]
  jade tasks help
  jade tasks <verb> help

VERBS
  list      List non-deleted tasks (ordered by due date)
  add       Create a new task
  update    Partially update an existing task
  delete    Soft-delete a task
  history   Show the task event log (newest first)

EXAMPLES
  jade tasks list
  jade tasks add \"Ship CLI\" --due next-monday
  jade tasks update --id <uuid> --status complete
  jade tasks delete --id <uuid>
  jade tasks history --id <uuid>

See also: jade tasks list help | add help | update help | delete help | history help
";

const TASKS_LIST: &str = "\
jade tasks list — list tasks

PURPOSE
  Print all non-deleted tasks ordered by due_at ascending.

USAGE
  jade tasks list [--json] [--db <path>]

OPTIONS
  --json        Pretty-printed JSON array of tasks
  --db <path>   Override database path

OUTPUT (default)
  Table columns: ID, STATUS, DUE, TITLE, TAGS

EXAMPLES
  jade tasks list
  jade tasks list --json
  jade tasks list --db ./tmp/jade.db

COMMON ERRORS
  (none typical) — empty DB prints \"(no tasks)\"
";

const TASKS_ADD: &str = "\
jade tasks add — create a task

PURPOSE
  Insert a new task. New tasks always start as status=inactive.

USAGE
  jade tasks add <title> [options]

ARGUMENTS
  <title>       Required task title (non-empty after trim)

OPTIONS
  -d, --description <text>   Optional description
  --due <value>              Due datetime (default: next whole local hour)
  -t, --tag <name>           Tag name (repeatable)
  --repeat <cron>            5-field POSIX cron (e.g. \"0 9 * * 1-5\")
  --json                     Print created task as JSON
  --db <path>                Override database path

DUE VALUES
  tomorrow                   Tomorrow, keeping a rounded time-of-day
  next-monday                Following Monday (skips today if Monday)
  <RFC3339>                  e.g. 2026-07-21T15:00:00-05:00
  YYYY-MM-DDTHH:MM[:SS]      Interpreted as local time
  YYYY-MM-DD                 Local noon on that date

EXAMPLES
  jade tasks add \"Buy milk\"
  jade tasks add \"Buy milk\" --due tomorrow --tag errands
  jade tasks add \"Review PR\" -d \"Check tests\" --due 2026-07-22T09:00
  jade tasks add \"Standup\" --due 2026-07-22T09:00 --repeat \"0 9 * * 1-5\"

COMMON ERRORS
  title is required          Empty / whitespace-only title
  invalid due date …         Unrecognized --due value
  invalid cron schedule …    Bad --repeat expression
";

const TASKS_UPDATE: &str = "\
jade tasks update — partial update

PURPOSE
  Change one or more fields on an existing task in a single command.
  At least one field flag is required.

USAGE
  jade tasks update --id <uuid> [--title …] [--description …] [--status …] [--due …] [--repeat …]

OPTIONS
  --id <uuid>                Required task id
  --title <text>             Replace title
  -d, --description <text>   Replace description (empty string clears it)
  --status <value>           inactive | active | complete
  --due <value>              tomorrow | next-monday | absolute datetime
  --repeat <cron|none>       5-field cron, or none to clear
  --json                     Print updated task as JSON
  --db <path>                Override database path

FIELD HELP
  jade tasks update status help
  jade tasks update due help
  jade tasks update title help
  jade tasks update description help

EXAMPLES
  jade tasks update --id <uuid> --status active
  jade tasks update --id <uuid> --due next-monday --title \"Buy oat milk\"
  jade tasks update --id <uuid> --description \"\"
  jade tasks update --id <uuid> --repeat \"0 9 * * *\"
  jade tasks update --id <uuid> --repeat none

COMMON ERRORS
  no fields to update        No field flags provided
  task not found: …          Unknown or already-deleted id
  title is required          --title was empty/whitespace
  invalid status: …          Bad --status value
  invalid cron schedule …    Bad --repeat expression
";

const TASKS_UPDATE_STATUS: &str = "\
jade tasks update --status — change task status

PURPOSE
  Move a task between the Inactive / Active / Complete lanes.

USAGE
  jade tasks update --id <uuid> --status <value>

ALLOWED VALUES
  inactive    Backlog / not started
  active      Currently in progress
  complete    Finished

STRUCTURE
  --status expects exactly one of the values above (snake_case, lowercase).
  It can be combined with other update flags in the same invocation.

EXAMPLES
  jade tasks update --id 11111111-1111-1111-1111-111111111111 --status active
  jade tasks update --id <uuid> --status complete --json

COMMON ERRORS
  invalid status: foo        Value not in the allowed set
  task not found: …          Unknown or deleted id
";

const TASKS_UPDATE_DUE: &str = "\
jade tasks update --due — reschedule a task

PURPOSE
  Change when a task is due. Presets are relative to the task's current due_at
  (same local time-of-day rules as the desktop app). Absolute values replace
  due_at outright.

USAGE
  jade tasks update --id <uuid> --due <value>

ALLOWED VALUES
  tomorrow                   Push calendar date +1 day, keep local time-of-day
  next-monday                Next Monday (if already Monday, jump a week)
  <RFC3339>                  Absolute instant, e.g. 2026-07-21T15:00:00Z
  YYYY-MM-DDTHH:MM[:SS]      Absolute local datetime
  YYYY-MM-DD                 Absolute local date at noon

EXAMPLES
  jade tasks update --id <uuid> --due tomorrow
  jade tasks update --id <uuid> --due next-monday
  jade tasks update --id <uuid> --due 2026-08-01T12:00

NOTES
  Presets use the existing due_at as the baseline (not \"now\"), matching
  jade-core reschedule helpers.

COMMON ERRORS
  invalid due date …         Unrecognized format
  task not found: …          Unknown or deleted id
";

const TASKS_UPDATE_TITLE: &str = "\
jade tasks update --title — rename a task

PURPOSE
  Replace the task title. Must be non-empty after trimming whitespace.

USAGE
  jade tasks update --id <uuid> --title <text>

EXAMPLES
  jade tasks update --id <uuid> --title \"Buy oat milk\"

COMMON ERRORS
  title is required          Empty / whitespace-only title
  task not found: …          Unknown or deleted id
";

const TASKS_UPDATE_DESCRIPTION: &str = "\
jade tasks update --description — set or clear description

PURPOSE
  Replace the optional description. Pass an empty string to clear it.

USAGE
  jade tasks update --id <uuid> --description <text>
  jade tasks update --id <uuid> -d <text>

EXAMPLES
  jade tasks update --id <uuid> --description \"Check dairy aisle\"
  jade tasks update --id <uuid> --description \"\"

COMMON ERRORS
  task not found: …          Unknown or deleted id
";

const TASKS_DELETE: &str = "\
jade tasks delete — soft-delete a task

PURPOSE
  Tombstone a task (sets deleted_at). It disappears from list and the board
  but remains in the database for future sync.

USAGE
  jade tasks delete --id <uuid> [--json] [--db <path>]

OPTIONS
  --id <uuid>     Required task id
  --json          Print {\"deleted\":true,\"id\":\"…\"}
  --db <path>     Override database path

EXAMPLES
  jade tasks delete --id <uuid>
  jade tasks delete --id <uuid> --json

COMMON ERRORS
  task not found: …          Unknown id or already deleted
";

const TASKS_HISTORY: &str = "\
jade tasks history — task event log

PURPOSE
  Show append-only events for task mutations (created, updated, deleted),
  newest first. Useful for auditing status changes and other edits.

USAGE
  jade tasks history [--id <uuid>] [--limit <n>] [--json] [--db <path>]

OPTIONS
  --id <uuid>     Filter to a single task (omit for all tasks)
  --limit <n>     Max events to return (default: 50)
  --json          Pretty-printed JSON array of events
  --db <path>     Override database path

OUTPUT (default)
  Table columns: WHEN, TASK, TYPE, CHANGES
  Updated events summarize field changes as field: old -> new.

EXAMPLES
  jade tasks history
  jade tasks history --id <uuid>
  jade tasks history --id <uuid> --limit 10 --json

NOTES
  Events are written for creates, updates (including status / due / tags /
  repeat), deletes, and recurring spawn creates (payload includes spawned_from).
";
