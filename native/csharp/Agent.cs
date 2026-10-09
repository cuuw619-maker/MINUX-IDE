namespace MINUX.Agent;

internal static class Program
{
    private static int Main(string[] args)
    {
        // NativeAOT console output is piped back to Rust; force UTF-8 to avoid
        // Windows OEM-codepage mojibake in Cyrillic responses.
        Console.InputEncoding = new System.Text.UTF8Encoding(false);
        Console.OutputEncoding = new System.Text.UTF8Encoding(false);

        if (args.Length == 1 && args[0] == "--version")
        {
            Console.WriteLine("1");
            return 0;
        }

        if (args.Length == 1 && args[0] == "--health")
        {
            Console.WriteLine("C# NativeAOT module is ready (UTF-8).");
            return 0;
        }

        var prompt = Console.In.ReadToEnd().Trim();
        if (string.IsNullOrWhiteSpace(prompt))
        {
            Console.WriteLine("Введите запрос.");
            return 0;
        }

        Console.WriteLine($"C# Agent получил запрос: {prompt}");
        Console.WriteLine();
        Console.WriteLine("This embedded C# component is a native extension host. Hugging Face requests are handled by the Rust host.");
        return 0;
    }
}
