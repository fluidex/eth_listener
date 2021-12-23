drop table block_log cascade;
create table block_log (
   id serial primary key,
   block_number bigint not null unique,
   created_at timestamp not null default current_timestamp
);
