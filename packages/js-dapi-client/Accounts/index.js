const User = require('./User/').User;
exports.Accounts = function(){
    let self = this;
    return {
        User: User()
    };
};