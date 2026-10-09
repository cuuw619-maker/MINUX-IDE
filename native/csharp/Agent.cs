namespace MINUX.Agent;

internal static class Program
{
    private static int Main(string[] args)
    {
        if (args.Length == 1 && args[0] == "--version")
        {
            Console.WriteLine("1");
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
        Console.WriteLine("AI-провайдер ещё не подключён. Сейчас агент подтверждает приём запроса; для реальных ответов потребуется подключить API.");
        return 0;
    }
}
