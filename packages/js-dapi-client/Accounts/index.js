const User = require('./User/').User;
const AuthService = require('./AuthService/authService').AuthService;

exports.Accounts = function() {
    return {
        API: {
            User: User(),
            AuthService: AuthService
        }
    }
};