const inMemDb = {
    challenges: {},
    getChallenge(key) {
        return this.challenges[key];
    },
    setChallenge(key, value) {
        this.challenges[key] = value;
    }
};

module.exports = inMemDb;