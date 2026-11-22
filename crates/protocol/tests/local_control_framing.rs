use myrig_protocol::local_control::framing::{read_frame, write_frame};
use std::io::ErrorKind;
use tokio::io::{AsyncWriteExt, duplex};

#[tokio::test]
async fn framing_roundtrip() {
    let (mut writer, mut reader) = duplex(64);
    write_frame(&mut writer, b"request", 1024).await.unwrap();
    assert_eq!(read_frame(&mut reader, 1024).await.unwrap(), b"request");
}

#[tokio::test]
async fn framing_rejects_zero_and_oversize() {
    for length in [0_u32, 1025] {
        let (mut writer, mut reader) = duplex(64);
        writer.write_all(&length.to_le_bytes()).await.unwrap();
        assert_eq!(
            read_frame(&mut reader, 1024).await.unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
}

#[tokio::test]
async fn framing_rejects_truncated_body() {
    let (mut writer, mut reader) = duplex(64);
    writer.write_all(&4_u32.to_le_bytes()).await.unwrap();
    writer.write_all(b"ab").await.unwrap();
    drop(writer);
    assert_eq!(
        read_frame(&mut reader, 1024).await.unwrap_err().kind(),
        ErrorKind::UnexpectedEof
    );
}

#[tokio::test]
async fn write_frame_refuses_empty_and_oversize_bodies() {
    let (mut writer, _) = duplex(64);
    for body in [&b""[..], b"12345"] {
        assert_eq!(
            write_frame(&mut writer, body, 4).await.unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
}
