config = {
    motd = [[
    Welcome to my MUD server
    Have a great time here!
    ]],

    listeners = {
        { bind = "0.0.0.0:7878" },
    },

    limits = {
        maxInFlightMsgs = 1024,
    },

    logging = {
        filter = "info",
},

    database = {
        host = "localhost",
        dbname = "tommo",
    },
}
