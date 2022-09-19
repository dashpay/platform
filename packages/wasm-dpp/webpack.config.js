const path = require('path');

module.exports = {
    entry: './index.ts',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'index.js',
        library: {
            type: 'umd'
        },
        publicPath: '',
        // This is needed to prevent ReferenceError: self is not defined,
        // as webpack names global object "self" for some reason
        globalObject: 'this'
    },
    mode: 'production',
    module: {
        rules: [
            {
                test: /\.tsx?$/,
                use: 'ts-loader',
                exclude: /node_modules/,
            },
        ],
    },
    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
        //fallback: { "util": false }
    },
    //target: 'node'
};
