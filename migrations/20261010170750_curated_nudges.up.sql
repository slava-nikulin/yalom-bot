CREATE TABLE curated_nudges (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    intent ENUM(
        'Observation',
        'Question',
        'Invitation',
        'Greeting'
    ) NOT NULL,
    daytime ENUM(
        'Any',
        'Morning',
        'Day',
        'Evening'
    ) NOT NULL DEFAULT 'Any',
    text TEXT NOT NULL,

    PRIMARY KEY (id),

    INDEX idx_curated_nudges_kind_daytime (
        intent,
        daytime
    )
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8mb4;