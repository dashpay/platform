# Logging

Wallet-lib will log multiple events happening which might help you to debug, or get a better understanding at what is happening internally. 

## Log levels 

These are the different levels used internally sorted from the least verbose to the most.
 
- `error` - Warn about issues that are near critical (but are not straight thrown errors).
- `warn` - Warn about issues that aren't so critical.
- `info` - Log level used by default. Inform about some internal high value steps.
- `debug` - Inform about basic steps (plugin initialisations, ...)
- `silly` - Inform about everything going on (each transporter call, storage and worker execution,...)

## Set a log level 

In order to control the granularity of the logger, simply put the environment variable `LOG_LEVEL` at the desired level. 

- Windows: `set LOG_LEVEL=silly node index.js`
- MacOS: `LOG_LEVEL=silly node index.js`
- Linux: `export LOG_LEVEL=silly node index.js` 
