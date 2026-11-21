ALTER TABLE journal ADD COLUMN audio_n INTEGER NOT NULL DEFAULT 0;
ALTER TABLE journal ADD COLUMN video_n INTEGER NOT NULL DEFAULT 0;

UPDATE journal
SET audio_n = (length(content) - length(replace(content, 'ddasset://audio/', ''))) / length('ddasset://audio/'),
    video_n = (length(content) - length(replace(content, 'ddasset://video/', ''))) / length('ddasset://video/');