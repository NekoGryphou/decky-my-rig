using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace MyRigConfig.Core.LocalControl;

// ? System.Text.Json escapes supplementary characters as surrogate pairs, even with its relaxed encoder. Writing UTF-8
// ? keeps the protocol's 80-character limit.
internal sealed class UnicodeNameConverter : JsonConverter<string>
{
    public override string? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString();
    public override void Write(Utf8JsonWriter writer, string value, JsonSerializerOptions options)
    {
        var escaped = value.Replace("\\", "\\\\").Replace("\"", "\\\"");
        writer.WriteRawValue(Encoding.UTF8.GetBytes("\"" + escaped + "\""), skipInputValidation: false);
    }
}
