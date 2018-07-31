create table commands(
    channel VARCHAR(60) NOT NULL,
    match_expr VARCHAR(200) NOT NULL,
    command VARCHAR(200) NOT NULL,

    primary key (channel, match_expr)
);