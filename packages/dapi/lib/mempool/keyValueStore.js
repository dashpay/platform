class KeyValueStore extends require('./mempoolBase') {
    constructor(namespace = 'dapinet') {
        super()
        this.kvs = this.orbitdb.kvstore(namespace)
        this.init();
    }

    init(key = 'message') {

        this.kvs.events.on('ready', () => {
            console.log("ready: " + this.kvs.get(key))
        })

        this.kvs.events.on('synced', () => {
            console.log("synced: " + this.kvs.get(key))
        })
    }

    writeValue(value, key = 'message') {
        this.kvs.set(key, value)
            .then(() => {
                console.log(this.kvs.get(key))
            })
    }

    getValue(key = 'message') {
        return this.kvs.get(key)
    }

}

module.exports = KeyValueStore
