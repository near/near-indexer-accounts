CREATE TABLE accounts
(
    account_id              varchar(64000) NOT NULL,
    created_by_receipt_id   varchar(64000),
    deleted_by_receipt_id   varchar(64000),
    created_by_block_height numeric(20, 0) NOT NULL,
    deleted_by_block_height numeric(20, 0)
);

CREATE TABLE access_keys
(
    public_key              varchar(64000) NOT NULL,
    account_id              varchar(64000) NOT NULL,
    created_by_receipt_id   varchar(64000),
    deleted_by_receipt_id   varchar(64000),
    created_by_block_height numeric(20, 0) NOT NULL,
    deleted_by_block_height numeric(20, 0),
    permission_kind         varchar(64000) NOT NULL
);
