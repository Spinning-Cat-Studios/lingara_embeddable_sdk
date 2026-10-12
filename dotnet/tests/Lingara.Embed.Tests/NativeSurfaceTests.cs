using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using Lingara.Embed.Native;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>The hand-written imports against <c>lingara_embed.h</c> (ADR 1.10.26aa D8): the drift guard csbindgen would have been.</summary>
    public class NativeSurfaceTests
    {
        private static readonly HashSet<Type> Blittable = new HashSet<Type>
        {
            typeof(byte*), typeof(UIntPtr*), typeof(IntPtr*), typeof(UIntPtr), typeof(IntPtr), typeof(int), typeof(uint), typeof(void),
        };

        /// <summary>Each declared function's name and parameter count.</summary>
        private static Dictionary<string, int> HeaderFunctions()
        {
            var header = File.ReadAllText(Support.Vector("lingara_embed.h"));
            var declaration = new Regex(@"\b(lingara_embed_\w+)\(([^)]*)\);");
            return declaration.Matches(header).ToDictionary(
                m => m.Groups[1].Value,
                m => m.Groups[2].Value.Trim() == "void" ? 0 : m.Groups[2].Value.Split(',').Length);
        }

        private static Dictionary<string, MethodInfo> Imports(Type type) =>
            type.GetMethods(BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public)
                .Where(m => (m.Attributes & MethodAttributes.PinvokeImpl) != 0)
                .ToDictionary(m => m.GetCustomAttribute<DllImportAttribute>()?.EntryPoint ?? m.Name);

        // 1.10.26aa AC14
        [Fact]
        public void EveryHeaderFunctionIsImportedInBothModes()
        {
            var header = HeaderFunctions();
            Assert.Equal(9, header.Count);
            foreach (var (type, library) in new[] { (typeof(NativeMethodsDynamic), "lingara_embed"), (typeof(NativeMethodsInternal), "__Internal") })
            {
                var imports = Imports(type);
                Assert.Equal(header.Keys.OrderBy(k => k), imports.Keys.OrderBy(k => k));
                foreach (var (name, method) in imports)
                {
                    var parameters = method.GetParameters();
                    Assert.True(header[name] == parameters.Length, $"{type.Name}.{name}: {parameters.Length} parameters, the header has {header[name]}");
                    Assert.All(parameters.Select(p => p.ParameterType).Append(method.ReturnType), t => Assert.Contains(t, Blittable));
                    var import = method.GetCustomAttribute<DllImportAttribute>()!;
                    Assert.Equal(library, import.Value);
                    Assert.Equal(CallingConvention.Cdecl, import.CallingConvention);
                }
            }
        }
    }
}
