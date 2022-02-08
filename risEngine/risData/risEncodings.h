#pragma once
#include "risPrimitives.h"

namespace risEngine
{
	typedef U32 CodePoint;

#pragma region UTF8
	// https://datatracker.ietf.org/doc/html/rfc3629

	template<typename CharType = U8>
	struct risUTF8
	{
		typedef CharType Character;

		template<typename OutputStream>
		static void encode(OutputStream& output_stream, CodePoint code_point)
		{
			if (code_point > 0x0010FFFF)
				return;

			if (code_point > 0x000FFFF)
			{
				output_stream.put(static_cast<Character>(0xF0 | (0x1C0000 & code_point) >> 18));
				output_stream.put(static_cast<Character>(0x80 | (0x3F000 & code_point) >> 12));
				output_stream.put(static_cast<Character>(0x80 | (0xFC0 & code_point) >> 6));
				output_stream.put(static_cast<Character>(0x80 | (0x3F & code_point)));
			}
			else if (code_point > 0x000007FF)
			{
				output_stream.put(static_cast<Character>(0xE0 | (0xF000 & code_point) >> 12));
				output_stream.put(static_cast<Character>(0x80 | (0xFC0 & code_point) >> 6));
				output_stream.put(static_cast<Character>(0x80 | (0x3F & code_point)));
			}
			else if (code_point > 0x0000007F)
			{
				output_stream.put(static_cast<Character>(0xC0 | (0x7C0 & code_point) >> 6));
				output_stream.put(static_cast<Character>(0x80 | (0x3F & code_point)));
			}
			else
			{
				output_stream.put(static_cast<Character>(0x0000007f & code_point));
			}
		}

		template<typename InputStream>
		static CodePoint decode(InputStream& input_stream)
		{
			Character byte1 = input_stream.take();
			if ((byte1 & 0x80) == 0)
			{
				return static_cast<CodePoint>(byte1);
			}

			if ((byte1 & 0xE0) == 0xC0)
			{
				Character byte2 = input_stream.take();
				if ((byte2 & 0xC0) != 0x80)
					return 0xFFFF;

				return (byte1 & 0x1F) << 6 | byte2 & 0x3F;
			}

			if ((byte1 & 0xF0) == 0xE0)
			{
				Character byte2 = input_stream.take();
				if ((byte2 & 0xC0) != 0x80)
					return 0xFFFF;

				Character byte3 = input_stream.take();
				if ((byte3 & 0xC0) != 0x80)
					return 0xFFFF;

				return (byte1 & 0x0F) << 12 | (byte2 & 0x3F) << 6 | byte3 & 0x3F;
			}

			if ((byte1 & 0xF8) == 0xF0)
			{
				Character byte2 = input_stream.take();
				if ((byte2 & 0xC0) != 0x80)
					return 0xFFFF;

				Character byte3 = input_stream.take();
				if ((byte3 & 0xC0) != 0x80)
					return 0xFFFF;

				Character byte4 = input_stream.take();
				if ((byte4 & 0xC0) != 0x80)
					return 0xFFFF;

				return (byte1 & 0x07) << 18 | (byte2 & 0x3F) << 12 | (byte3 & 0x3F) << 6 | byte4 & 0x3F;
			}

			return 0xFFFF;
		}
	};
#pragma endregion

#pragma region UTF16
	// https://datatracker.ietf.org/doc/html/rfc2781

	template<typename CharType = wchar_t>
	struct risUTF16LE
	{
		typedef CharType Character;

		template<typename OutputStream>
		static void encode(OutputStream& output_stream, CodePoint code_point)
		{
			output_stream.put(static_cast<Character>(code_point));
		}

		template<typename InputStream>
		static CodePoint decode(InputStream& input_stream)
		{
			return input_stream.take();
		}
	};

	template<typename CharType = wchar_t>
	struct risUTF16BE
	{
		typedef CharType Character;

		template<typename OutputStream>
		static void encode(OutputStream& output_stream, CodePoint code_point)
		{
			output_stream.put(static_cast<Character>(code_point));
		}

		template<typename InputStream>
		static CodePoint decode(InputStream& input_stream)
		{
			return input_stream.take();
		}
	};

	template<typename CharType = wchar_t>
	struct risUTF16
	{
		typedef CharType Character;

		template<typename OutputStream>
		static void encode(OutputStream& output_stream, CodePoint code_point)
		{
			if (code_point < 0x10000)
			{
				output_stream.put(static_cast<Character>(code_point));
			}
			else
			{
				const CodePoint shifted_code_point = code_point - 0x10000;

				output_stream.put(static_cast<Character>(0xD800 | (0xFFC00 & shifted_code_point) >> 10));
				output_stream.put(static_cast<Character>(0xDC00 | (0x3FF & shifted_code_point)));
			}
		}

		template<typename InputStream>
		static CodePoint decode(InputStream& input_stream)
		{
			Character W1 = input_stream.take();
			if (W1 < 0xD800 || W1 > 0xDFFF)
				return static_cast<CodePoint>(W1);

			if (W1 <= 0xD800 || W1 >= 0xDBFF)
				return 0xFFFF;

			Character W2 = input_stream.take();
			if (W2 == 0)
				return 0xFFFF;

			return (W1 & 0x3FF) << 10 | W2 & 0x3FF;
		}
	};
#pragma endregion

#pragma region ASCII
	template<typename CharType = char>
	struct risASCII
	{
		typedef CharType Character;

		template<typename OutputStream>
		static void encode(OutputStream& output_stream, CodePoint code_point)
		{
			output_stream.put(static_cast<Character>(code_point & 0x7F));
		}

		template<typename InputStream>
		static CodePoint decode(InputStream& input_stream)
		{
			return input_stream.take() & 0x7F;
		}
	};
#pragma endregion
}