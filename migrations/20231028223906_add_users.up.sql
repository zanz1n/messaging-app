CREATE TYPE "userrole" AS ENUM('ADMIN', 'COMMON');

CREATE TABLE "users" (
    "id" uuid PRIMARY KEY,
    "created_at" timestamptz(3) NOT NULL DEFAULT current_timestamp,
    "updated_at" timestamptz(3) NOT NULL DEFAULT current_timestamp,
    "email" varchar(64) NOT NULL,
    "username" varchar(32) NOT NULL,
    "role" "userrole" NOT NULL DEFAULT 'COMMON',
    "password" varchar(60) NOT NULL
);
