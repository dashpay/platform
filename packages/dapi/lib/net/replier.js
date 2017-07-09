class Replier extends require('./send_socket') {
    constructor(params) {
        super(params);
    }

    attach() {
        super.attach('rep');
    }
}

module.exports = Replier;