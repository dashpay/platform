Commands located at `./commands` folder.

Guideline for writing rpc commands:

- No http calls in commands. Move http calls to separate api classes
- Always use `try catch` in command body, as it is last step where error can be caught;
- Always return error to command callback properly