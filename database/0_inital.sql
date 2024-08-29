CREATE TABLE players (
    id SERIAL NOT NULL,
    uuid uuid NOT NULL,
    creation_time timestamp without time zone NOT NULL,
    last_connection_time timestamp without time zone,
    nickname character varying(16) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (nickname),
    CONSTRAINT "Nickname isn't empty" CHECK ((length((nickname)::text) > 0))
);

CREATE INDEX players_uuid ON players USING hash (uuid);

CREATE TABLE player_permissions (
    player_id integer NOT NULL,
    permission character varying NOT NULL,
    PRIMARY KEY (player_id),
    UNIQUE (player_id, permission),
    FOREIGN KEY (player_id)
        REFERENCES players (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
        NOT VALID
);

CREATE TABLE player_ships (
    player_id integer NOT NULL,
    slot integer NOT NULL,
    last_update timestamp without time zone NOT NULL,
    data jsonb NOT NULL,
    PRIMARY KEY (player_id, slot),
    FOREIGN KEY (player_id)
        REFERENCES players (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
        NOT VALID
);

CREATE TABLE player_tokens (
    token character varying NOT NULL,
    player_id integer NOT NULL,
    PRIMARY KEY (token),
    FOREIGN KEY (player_id)
        REFERENCES players (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
        NOT VALID
);
