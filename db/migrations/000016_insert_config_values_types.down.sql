USE de_releases;

DELETE FROM config_value_types WHERE name = 'string';
DELETE FROM config_value_types WHERE name = 'int';
DELETE FROM config_value_types WHERE name = 'bigint';
DELETE FROM config_value_types WHERE name = 'bool';
DELETE FROM config_value_types WHERE name = 'float';
DELETE FROM config_value_types WHERE name = 'json';
DELETE FROM config_value_types WHERE name = 'yaml';
DELETE FROM config_value_types WHERE name = 'xml';
DELETE FROM config_value_types WHERE name = 'csv';
DELETE FROM config_value_types WHERE name = 'tsv';