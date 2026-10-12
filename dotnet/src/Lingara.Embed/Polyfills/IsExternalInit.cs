#if !NET5_0_OR_GREATER
namespace System.Runtime.CompilerServices
{
    /// <summary>
    /// The marker C# 9's <c>init</c> accessors need, which netstandard2.1 lacks.
    /// The mirrored types are sealed classes with init-only properties in place of
    /// records (ADR 1.10.26aa D8).
    /// </summary>
    internal static class IsExternalInit
    {
    }
}
#endif
