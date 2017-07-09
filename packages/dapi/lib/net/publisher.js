class Publisher extends require('./send_socket') {
    constructor(params) {
        super(params);
    }

    attach() {
        super.attach('pub');
    }

}

module.exports = Publisher;