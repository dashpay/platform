const User = function() {
    return {
        create: require('./create').create,
        login: require('./login').login,
        search: require('./search').search,
        send: require('./send').send,
        update: require('./update').update,
        remove: require('./remove').remove,
    }
};

exports.User = User;