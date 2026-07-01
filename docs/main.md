# Welcome to the board language docs!

[board](./board.md)
[config file](./config.md)
[settings file](./settings.md)

## What is board language?
Board language is a user frandly way of creating kanban board style.

file example:
```bd {fillename="example.bd"}
[PROFILES]
Github
Example

[BOARDS]
To do #911db4
Bug #b41d1d +code
Doing #c2bf1e

[TASKS]
** To do - Do the dishes +kitchen @house !24/5/2026 #ffffff [How to do it](https://howtododishes.com/) [What to do](./test.md)
* Doing - Programming +Computer rgba(29, 67, 180, 0.5)
11* Done - Presentation @School !11/5/2026 rgb(255, 0, 0)
```

As you can see, this language was designed to be readable first taking some ideas of how normally people organize their stuff in markdown, plain text or ogs.

lets take the task:
** [BOARD] - [LABEL] +[TAG_0] @[CONTEXT_0] ![DUE] [COLOR (rgb, rgba or hex)] [LINK_NAME][LINK_0]

Organization:
- "*": represents the priority the task has, being able to also write amount + "*" (***) or amount * "*" (3*)
- [BOARD]: what board the task is
- [LABEL]: task label
- ...