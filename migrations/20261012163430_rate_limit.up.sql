CREATE TABLE manual_nudge_rate_limits (
    tg_user_id BIGINT NOT NULL,

    tat_utc DATETIME(6) NULL,

    window_start_utc DATETIME(6) NOT NULL,
    request_count INT UNSIGNED NOT NULL DEFAULT 0,

    PRIMARY KEY (tg_user_id),

    CONSTRAINT fk_manual_nudge_rate_limits_user
        FOREIGN KEY (tg_user_id)
        REFERENCES users (tg_user_id)
        ON DELETE CASCADE
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8mb4;